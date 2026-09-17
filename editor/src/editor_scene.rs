use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use libdqg::camera::Camera;
use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::input::{InputState, MouseState};
use libdqg::renderer::{DrawPass, Renderer};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::scripting::{ScriptError, ScriptList, ScriptRuntime};
use libdqg::types::Color;
use libdqg::world::{CameraComponent, Renderable, Transform, World};

use crate::editor_settings::EditorSettings;
use crate::egui_layer::EguiLayer;
use crate::fly_camera::FlyCamera;
use crate::picking;
use crate::project::{EntityRecord, Project, RenderableAsset, RenderableKind};
use crate::recent_projects::RecentProjects;
use crate::ui;

/// Longest a single `update` is allowed to spend loading entity renderables before yielding a
/// frame back to the event loop, so the loading screen keeps animating and the window stays
/// responsive on a project with a lot of assets instead of freezing until it's all done.
const LOAD_BUDGET_PER_FRAME: Duration = Duration::from_millis(8);

/// How long a script error stays in the overlay (see [`EditorScene::script_errors`]) before it
/// ages out on its own, so a one-off error doesn't linger forever if the user doesn't hit Stop.
const SCRIPT_ERROR_DISPLAY: Duration = Duration::from_secs(6);

/// Whether the editor is authoring the scene or running it live. Play snapshots the whole
/// [`World`] and starts a [`ScriptRuntime`]; Stop restores the snapshot and drops the runtime
/// (and every script's state with it) — see [`EditorScene::start_play`]/[`stop_play`]. A
/// whole-`World` snapshot (not just `Transform`) is what makes scripts that spawn/despawn/rename
/// entities or attach scripts fully revert on Stop, the same way a Transform-only snapshot
/// already made translate/rotate/scale revert.
enum EditorMode {
    Edit,
    Playing { world_snapshot: World, runtime: ScriptRuntime },
}

/// What the editor scene should do with a project root on its first `update`, once a
/// `Renderer` (and therefore GPU access for loading assets) is available. Set either by the
/// menu scene handing off to a freshly-built `EditorScene`, or by the editor's own File menu.
pub enum PendingAction {
    New(PathBuf),
    Open(PathBuf),
}

/// Where a project load currently stands, so it can be spread across several frames instead of
/// blocking the main thread for its whole duration.
enum LoadState {
    /// Requested but not started; the project itself hasn't been created/opened yet.
    Pending(PendingAction),
    /// The project is resolved and its scene's entities are queued; `remaining` is drained a
    /// few at a time (see [`LOAD_BUDGET_PER_FRAME`]) each frame until empty.
    LoadingAssets { project: Project, remaining: VecDeque<EntityRecord>, total: usize },
}

pub struct EditorScene {
    world: World,
    selected: Option<Entity>,
    /// The entity currently under the mouse cursor in the 3D viewport, if any. Recomputed every
    /// frame (unlike `selected`, which only changes on click) so the hovered model can blink.
    hovered: Option<Entity>,
    /// Elapsed time accumulator driving the hover blink, in seconds.
    hover_blink_time: f32,
    renaming: Option<Entity>,
    rename_buffer: String,
    project: Option<Project>,
    entity_assets: HashMap<Entity, RenderableAsset>,
    fly_camera: FlyCamera,
    last_mouse_pos: (f32, f32),
    recent: RecentProjects,
    load: Option<LoadState>,
    egui: EguiLayer,
    assets_expanded: bool,
    texture_previews: HashMap<PathBuf, egui::TextureHandle>,
    mode: EditorMode,
    /// Recent script compile/runtime errors and when each was recorded, for the overlay in
    /// [`ui::draw_script_error_overlay`] — pruned by [`SCRIPT_ERROR_DISPLAY`] each frame and
    /// cleared outright on Stop.
    script_errors: Vec<(String, Instant)>,
    /// Editor-wide preferences (currently just the external editor "Open Script" launches),
    /// persisted next to the executable like [`RecentProjects`].
    settings: EditorSettings,
    /// The script currently being renamed inline in the Assets panel, if any — mirrors
    /// `renaming`/`rename_buffer` above, just keyed by a script's project-relative path instead
    /// of an [`Entity`] since scripts aren't ECS entities.
    renaming_script: Option<PathBuf>,
    script_rename_buffer: String,
}

impl EditorScene {
    /// Builds a scene that will create or open `action`'s project root on its first `update`.
    pub fn opening(action: PendingAction) -> Self {
        let mut camera = Camera {
            position: glam::Vec3::new(0.0, 1.5, 4.0),
            yaw: 0.0,
            pitch: 0.0,
            aspect: 1.0,
            fov: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        camera.look_at(glam::Vec3::ZERO);

        Self {
            world: World::new(camera),
            selected: None,
            hovered: None,
            hover_blink_time: 0.0,
            renaming: None,
            rename_buffer: String::new(),
            project: None,
            entity_assets: HashMap::new(),
            fly_camera: FlyCamera::new(6.0),
            last_mouse_pos: (0.0, 0.0),
            recent: RecentProjects::load(),
            load: Some(LoadState::Pending(action)),
            egui: EguiLayer::new(),
            assets_expanded: true,
            texture_previews: HashMap::new(),
            mode: EditorMode::Edit,
            script_errors: Vec::new(),
            settings: EditorSettings::load(),
            renaming_script: None,
            script_rename_buffer: String::new(),
        }
    }

    /// Queues `action` to start on the next `update`.
    fn queue_action(&mut self, action: PendingAction) {
        self.load = Some(LoadState::Pending(action));
    }

    /// Resolves `action` into an open [`Project`] (fast: just filesystem/manifest/scene-file
    /// work, no asset decoding), replacing the current world with an empty one and queuing the
    /// saved scene's entities — if any — to be loaded incrementally afterward. Records the
    /// project in the recent-projects list.
    fn start_load(&mut self, action: PendingAction) {
        let (root, create) = match action {
            PendingAction::New(root) => (root, true),
            PendingAction::Open(root) => (root, false),
        };

        let result = if create { Project::create(root.clone()) } else { Project::open(root.clone()) };
        let project = match result {
            Ok(project) => project,
            Err(e) => {
                eprintln!("Failed to {} project: {e}", if create { "create" } else { "open" });
                return;
            }
        };

        self.world = World::new(self.world.camera);
        self.entity_assets.clear();
        self.selected = None;
        self.hovered = None;
        self.texture_previews.clear();
        // A live ScriptRuntime holds Entity handles into the *old* World; opening a different
        // project out from under it would leave it pointing at nothing, so just stop Play first.
        self.mode = EditorMode::Edit;
        self.script_errors.clear();

        let entities: VecDeque<EntityRecord> = if create {
            VecDeque::new()
        } else {
            match project.load_scene() {
                Ok(scene_file) => scene_file.entities.into(),
                Err(e) => {
                    eprintln!("Failed to load scene: {e}");
                    VecDeque::new()
                }
            }
        };

        self.recent.add(root);
        let _ = self.recent.save();

        if entities.is_empty() {
            self.project = Some(project);
        } else {
            let total = entities.len();
            self.load = Some(LoadState::LoadingAssets { project, remaining: entities, total });
        }
    }

    /// Loads entity renderables off `remaining` for up to [`LOAD_BUDGET_PER_FRAME`], then either
    /// puts the rest back for the next frame or, once it's empty, finishes the load.
    fn continue_load(&mut self, project: Project, mut remaining: VecDeque<EntityRecord>, total: usize, renderer: &Renderer) {
        let deadline = Instant::now() + LOAD_BUDGET_PER_FRAME;
        while Instant::now() < deadline {
            let Some(record) = remaining.pop_front() else { break };
            let entity = self.world.spawn_empty(record.name, record.transform);
            if let Some(asset) = record.renderable {
                match asset.load(renderer, &project) {
                    Ok(renderable) => {
                        self.world.set_renderable(entity, renderable);
                        self.entity_assets.insert(entity, asset);
                    }
                    Err(e) => eprintln!("Failed to load renderable: {e}"),
                }
            }
            if !record.scripts.is_empty() {
                self.world.scripts.insert(entity, ScriptList(record.scripts));
            }
            if let Some(component) = record.camera {
                self.world.set_camera(entity, component);
            }
        }

        if remaining.is_empty() {
            self.project = Some(project);
        } else {
            self.load = Some(LoadState::LoadingAssets { project, remaining, total });
        }
    }

    /// Snapshots the whole [`World`] and starts every enabled script attachment (in order) on
    /// every entity that has one, then switches to [`EditorMode::Playing`]. A no-op if there's no
    /// open project (the Play button is disabled in that case anyway — see `ui::draw_menu_bar`).
    fn start_play(&mut self) {
        let Some(project) = self.project.as_ref() else { return };

        let world_snapshot = self.world.clone();
        let mut runtime = ScriptRuntime::new(project.root.clone());
        // Without this, `world.find(...)` inside an `on_start` hook would see nothing but a
        // default-empty snapshot (only populated per-frame from here on, right before
        // `update_entity`) and always return `()` — `on_start` gets the same start-of-Play
        // snapshot guarantee `on_update` already has for cross-entity reads.
        runtime.begin_frame(&self.world);

        for entity in self.world.iter_entities().collect::<Vec<_>>() {
            let Some(list) = self.world.scripts.get(entity) else { continue };
            let starts: Vec<(usize, PathBuf)> = list
                .0
                .iter()
                .enumerate()
                .filter(|(_, attachment)| attachment.enabled)
                .map(|(index, attachment)| (index, project.root.join(&attachment.path)))
                .collect();

            for (index, script_path) in starts {
                if let Err(e) = runtime.start_script(&self.world, entity, index, &script_path) {
                    self.script_errors.push((format_script_error(&e), Instant::now()));
                }
            }
        }

        self.mode = EditorMode::Playing { world_snapshot, runtime };
    }

    /// Restores the whole [`World`] [`start_play`](Self::start_play) snapshotted and drops the
    /// [`ScriptRuntime`] (and with it every script's persistent state), switching back to
    /// [`EditorMode::Edit`].
    fn stop_play(&mut self) {
        if let EditorMode::Playing { world_snapshot, .. } = std::mem::replace(&mut self.mode, EditorMode::Edit) {
            self.world = world_snapshot;
        }
        self.script_errors.clear();
    }
}

fn format_script_error(error: &ScriptError) -> String {
    format!("{}: {}", error.script.display(), error.message)
}

/// Fixed visual size for a camera entity's frustum gizmo, in world units — deliberately not
/// derived from the component's real `znear`/`zfar` (which can be arbitrarily large), since the
/// gizmo is an at-a-glance orientation indicator, not a literal clip-volume outline.
const CAMERA_GIZMO_DISTANCE: f32 = 0.6;
/// Placeholder aspect ratio for the gizmo's proportions — `CameraComponent` doesn't store aspect
/// (it's viewport-derived, only meaningful once Play assigns a real camera), so the gizmo just
/// assumes a common 16:9 shape.
const CAMERA_GIZMO_ASPECT: f32 = 16.0 / 9.0;

/// Draws an 8-line wireframe pyramid (eye to 4 far-plane corners, plus the far rectangle)
/// representing `component`'s frustum at `transform`, projected through `view_camera` (the
/// camera currently driving the viewport).
fn draw_camera_gizmo(pass: &mut DrawPass, view_camera: &Camera, transform: &Transform, component: &CameraComponent, color: Color) {
    let forward = transform.rotation * glam::Vec3::NEG_Z;
    let up = transform.rotation * glam::Vec3::Y;
    let right = transform.rotation * glam::Vec3::X;

    let half_height = (component.fov.to_radians() * 0.5).tan() * CAMERA_GIZMO_DISTANCE;
    let half_width = half_height * CAMERA_GIZMO_ASPECT;

    let eye = transform.position;
    let far_center = eye + forward * CAMERA_GIZMO_DISTANCE;
    let corners = [
        far_center + up * half_height + right * half_width,
        far_center + up * half_height - right * half_width,
        far_center - up * half_height - right * half_width,
        far_center - up * half_height + right * half_width,
    ];

    for corner in corners {
        pass.draw_world_line(view_camera, eye, corner, 1.5, color);
    }
    for i in 0..4 {
        pass.draw_world_line(view_camera, corners[i], corners[(i + 1) % 4], 1.5, color);
    }
}

impl Scene for EditorScene {
    fn update(
        &mut self,
        delta_time: f32,
        input_state: &InputState,
        mouse_state: &MouseState,
        renderer: Option<&mut Renderer>,
    ) -> SceneTransition {
        let Some(renderer) = renderer else { return SceneTransition::None };

        if let Some(state) = self.load.take() {
            match state {
                LoadState::Pending(action) => self.start_load(action),
                LoadState::LoadingAssets { project, remaining, total } => {
                    self.continue_load(project, remaining, total, renderer);
                }
            }

            if let Some(LoadState::LoadingAssets { remaining, total, .. }) = &self.load {
                let loaded = total - remaining.len();
                let total = *total;
                self.egui.run(renderer, |ui| draw_loading_screen(ui, loaded, total));
                return SceneTransition::None;
            }
        }

        let mouse_pos = mouse_state.position();
        let mouse_delta = (mouse_pos.0 - self.last_mouse_pos.0, mouse_pos.1 - self.last_mouse_pos.1);
        self.last_mouse_pos = mouse_pos;

        self.script_errors.retain(|(_, at)| at.elapsed() < SCRIPT_ERROR_DISPLAY);
        let is_playing = matches!(self.mode, EditorMode::Playing { .. });
        let script_error_messages: Vec<String> = self.script_errors.iter().map(|(message, _)| message.clone()).collect();

        let world = &mut self.world;
        let selected = &mut self.selected;
        let renaming = &mut self.renaming;
        let rename_buffer = &mut self.rename_buffer;
        let project = self.project.as_ref();
        let entity_assets = &mut self.entity_assets;
        let assets_expanded = &mut self.assets_expanded;
        let texture_previews = &mut self.texture_previews;
        let settings = &mut self.settings;
        let renaming_script = &mut self.renaming_script;
        let script_rename_buffer = &mut self.script_rename_buffer;
        let mut requests = ui::UiRequests::default();
        self.egui.run(renderer, |ui| {
            ui::draw(
                ui,
                world,
                selected,
                renaming,
                rename_buffer,
                project,
                entity_assets,
                assets_expanded,
                texture_previews,
                is_playing,
                &script_error_messages,
                settings,
                renaming_script,
                script_rename_buffer,
                &mut requests,
            );
        });

        if let Some(root) = requests.new_project.take() {
            self.queue_action(PendingAction::New(root));
        }

        if let Some(root) = requests.open_project.take() {
            self.queue_action(PendingAction::Open(root));
        }

        if requests.toggle_play {
            if is_playing {
                self.stop_play();
            } else {
                self.start_play();
            }
        }

        if let Some((entity, kind, path)) = requests.attach_renderable.take() {
            if let Some(project) = self.project.as_ref() {
                let probe_asset = match kind {
                    RenderableKind::Sprite => {
                        RenderableAsset::Sprite { texture_path: path.clone(), width: 0.0, height: 0.0 }
                    }
                    RenderableKind::Model => RenderableAsset::Model { model_path: path.clone() },
                };
                match probe_asset.load(renderer, project) {
                    Ok(renderable) => {
                        // Re-derive the sprite's width/height from the loaded texture's native
                        // size rather than trusting the placeholder above.
                        let asset = match &renderable {
                            Renderable::Sprite(sprite) => {
                                RenderableAsset::Sprite { texture_path: path, width: sprite.width, height: sprite.height }
                            }
                            Renderable::Model(_) => RenderableAsset::Model { model_path: path },
                        };
                        self.world.set_renderable(entity, renderable);
                        self.entity_assets.insert(entity, asset);
                    }
                    Err(e) => eprintln!("Failed to attach renderable: {e}"),
                }
            }
        }

        self.hover_blink_time += delta_time;

        match &mut self.mode {
            EditorMode::Edit => {
                if !self.egui.wants_pointer_input() {
                    self.fly_camera.update(delta_time, input_state, mouse_state, &mut self.world.camera, mouse_delta);

                    let size = renderer.size();
                    let ndc_x = (mouse_pos.0 / (size.width.max(1) as f32)) * 2.0 - 1.0;
                    let ndc_y = 1.0 - (mouse_pos.1 / (size.height.max(1) as f32)) * 2.0;
                    self.hovered = picking::pick(&self.world, &self.world.camera, ndc_x, ndc_y);

                    if mouse_state.is_button_pressed(winit::event::MouseButton::Left) {
                        self.selected = self.hovered;
                    }
                } else {
                    self.hovered = None;
                }
            }
            EditorMode::Playing { runtime, .. } => {
                // Camera flying and viewport picking are edit-mode-only for v1 — playing just
                // runs scripts against the live World; see the implementation plan's
                // "Camera/picking fully disabled during Play" note.
                self.hovered = None;
                runtime.begin_frame(&self.world);
                for entity in self.world.iter_entities().collect::<Vec<_>>() {
                    for error in runtime.update_entity(&mut self.world, entity, delta_time, input_state, mouse_state, mouse_delta, Some(&*renderer)) {
                        self.script_errors.push((format_script_error(&error), Instant::now()));
                    }
                }
            }
        }

        self.world.sync_transforms();
        let size = renderer.size();
        let aspect = size.width as f32 / (size.height.max(1) as f32);
        self.world.camera.aspect = aspect;

        let push_camera = if is_playing {
            self.world.active_camera(aspect).unwrap_or(self.world.camera)
        } else {
            self.world.camera
        };
        *renderer.camera_mut() = push_camera;

        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        let blink_alpha = 0.15 + 0.35 * (self.hover_blink_time * 6.0).sin().abs();
        let hover_highlight = Color::new(1.0, 1.0, 1.0, blink_alpha as f64);
        let no_highlight = Color::new(1.0, 1.0, 1.0, 0.0);

        for (entity, renderable) in self.world.renderables.iter_mut() {
            let highlight = if self.hovered == Some(entity) { hover_highlight } else { no_highlight };
            let is_selected = self.selected == Some(entity);

            match renderable {
                Renderable::Sprite(sprite) => {
                    sprite.highlight = highlight;
                    // The outline is coplanar with the sprite and relies on draw order (not
                    // depth testing) to stay confined to a border — see
                    // `draw_world_sprite_outline`'s doc comment — so it must be drawn first.
                    if is_selected {
                        pass.draw_world_sprite_outline(sprite.x, sprite.y, sprite.z, sprite.width, sprite.height, sprite.transform);
                    }
                    sprite.draw_world(pass);
                }
                Renderable::Model(model) => {
                    model.highlight = highlight;
                    pass.draw_model(model);
                    if is_selected {
                        pass.draw_model_outline(model);
                    }
                }
            }
        }

        if matches!(self.mode, EditorMode::Edit) {
            for (entity, component) in self.world.cameras.iter() {
                let Some(transform) = self.world.transforms.get(entity) else { continue };
                let color = if self.selected == Some(entity) {
                    Color::new(0.3, 0.7, 1.0, 1.0)
                } else {
                    Color::new(0.6, 0.6, 0.6, 1.0)
                };
                draw_camera_gizmo(pass, &self.world.camera, transform, component, color);
            }
        }
    }

    fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        self.egui.on_window_event(event);
    }

    fn render_overlay(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.egui.render_overlay(device, queue, encoder, view);
    }
}

fn draw_loading_screen(ui: &mut egui::Ui, loaded: usize, total: usize) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.spinner();
                ui.add_space(8.0);
                ui.label(format!("Loading project... ({loaded}/{total})"));
            });
        });
    });
}
