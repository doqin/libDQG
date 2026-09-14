use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use libdqg::camera::Camera;
use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::input::{InputState, MouseState};
use libdqg::renderer::{DrawPass, Renderer};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::world::{Renderable, World};

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
}

impl EditorScene {
    /// Builds a scene that will create or open `action`'s project root on its first `update`.
    pub fn opening(action: PendingAction) -> Self {
        let camera = Camera {
            eye: glam::Vec3::new(0.0, 1.5, 4.0),
            target: glam::Vec3::ZERO,
            up: glam::Vec3::Y,
            aspect: 1.0,
            fov: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        Self {
            world: World::new(camera),
            selected: None,
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
        self.texture_previews.clear();

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
        }

        if remaining.is_empty() {
            self.project = Some(project);
        } else {
            self.load = Some(LoadState::LoadingAssets { project, remaining, total });
        }
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

        let world = &mut self.world;
        let selected = &mut self.selected;
        let renaming = &mut self.renaming;
        let rename_buffer = &mut self.rename_buffer;
        let project = self.project.as_ref();
        let entity_assets = &mut self.entity_assets;
        let assets_expanded = &mut self.assets_expanded;
        let texture_previews = &mut self.texture_previews;
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
                &mut requests,
            );
        });

        if let Some(root) = requests.new_project.take() {
            self.queue_action(PendingAction::New(root));
        }

        if let Some(root) = requests.open_project.take() {
            self.queue_action(PendingAction::Open(root));
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

        if !self.egui.wants_pointer_input() {
            self.fly_camera.update(delta_time, input_state, mouse_state, &mut self.world.camera, mouse_delta);

            if mouse_state.is_button_pressed(winit::event::MouseButton::Left) {
                let size = renderer.size();
                let ndc_x = (mouse_pos.0 / (size.width.max(1) as f32)) * 2.0 - 1.0;
                let ndc_y = 1.0 - (mouse_pos.1 / (size.height.max(1) as f32)) * 2.0;
                self.selected = picking::pick(&self.world, &self.world.camera, ndc_x, ndc_y);
            }
        }

        self.world.sync_transforms();
        let size = renderer.size();
        self.world.camera.aspect = size.width as f32 / (size.height.max(1) as f32);
        *renderer.camera_mut() = self.world.camera;

        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        for (_, renderable) in self.world.renderables.iter() {
            match renderable {
                Renderable::Sprite(sprite) => sprite.draw_world(pass),
                Renderable::Model(model) => pass.draw_model(model),
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
