use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use libdqg::camera::Camera;
use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::input::{InputState, MouseState};
use libdqg::renderer::{DrawPass, Renderer};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::scripting::{ScriptError, ScriptRuntime};
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

/// How long the last Export result stays in the overlay (see [`EditorScene::export_status`])
/// before it ages out on its own.
const EXPORT_STATUS_DISPLAY: Duration = Duration::from_secs(6);

/// Whether the editor is authoring the scene or running it live. Play snapshots the whole
/// [`World`] and starts a [`ScriptRuntime`]; Stop restores the snapshot and drops the runtime
/// (and every script's state with it) — see [`EditorScene::start_play`]/[`stop_play`]. A
/// whole-`World` snapshot (not just `Transform`) is what makes scripts that spawn/despawn/rename
/// entities or attach scripts fully revert on Stop, the same way a Transform-only snapshot
/// already made translate/rotate/scale revert. Always targets whichever tab was active when Play
/// started — see [`EditorScene::switch_or_open_scene`]/[`close_scene_tab`] for why switching or
/// closing tabs stops Play first.
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

/// Where a load currently stands, so it can be spread across several frames instead of blocking
/// the main thread for its whole duration.
enum LoadState {
    /// Requested but not started; the project itself hasn't been created/opened yet.
    Pending(PendingAction),
    /// A New/Open Project action resolved into `project`; `remaining` entities for its start
    /// scene (if any) are drained a few at a time (see [`LOAD_BUDGET_PER_FRAME`]) into `world`/
    /// `entity_assets` each frame until empty, at which point this *replaces* `open_scenes`
    /// wholesale — a different project's tabs aren't valid to keep around.
    LoadingProject {
        project: Project,
        path: PathBuf,
        world: World,
        entity_assets: HashMap<Entity, RenderableAsset>,
        remaining: VecDeque<EntityRecord>,
        total: usize,
    },
    /// An additional scene in the *current* project is being opened as a new tab; `remaining` is
    /// drained the same way, then the finished [`OpenScene`] is appended to `open_scenes` and
    /// made active — every other already-open tab is left untouched throughout.
    LoadingScene {
        path: PathBuf,
        world: World,
        entity_assets: HashMap<Entity, RenderableAsset>,
        remaining: VecDeque<EntityRecord>,
        total: usize,
    },
}

/// Spawns up to [`LOAD_BUDGET_PER_FRAME`] worth of `remaining` entities into `world`/
/// `entity_assets`, returning `true` once `remaining` is empty (the load is finished) — shared by
/// the "New/Open Project" and "open an additional scene as a new tab" load paths.
fn spawn_budgeted(
    world: &mut World,
    entity_assets: &mut HashMap<Entity, RenderableAsset>,
    remaining: &mut VecDeque<EntityRecord>,
    renderer: &Renderer,
    project_root: &Path,
) -> bool {
    let deadline = Instant::now() + LOAD_BUDGET_PER_FRAME;
    while Instant::now() < deadline {
        let Some(record) = remaining.pop_front() else { break };
        let (entity, asset) = record.spawn_into(world, Some(renderer), project_root);
        if let Some(asset) = asset {
            entity_assets.insert(entity, asset);
        }
    }
    remaining.is_empty()
}

/// One open scene tab: its own `World`, entity-asset tracking, hierarchy/inspector selection
/// state, and viewport camera — everything that used to be a single set of fields directly on
/// `EditorScene` before a project could have more than one scene open for editing at once. Kept
/// fully loaded in memory for as long as its tab stays open (see `EditorScene::open_scenes`),
/// unlike the old single-scene design, which discarded and reloaded a scene's `World` from disk
/// on every switch.
struct OpenScene {
    /// Project-relative path, e.g. `"scenes/main.ron"` — this tab's identity; a scene can only be
    /// open in one tab at a time (see `EditorScene::switch_or_open_scene`).
    path: PathBuf,
    world: World,
    entity_assets: HashMap<Entity, RenderableAsset>,
    selected: Option<Entity>,
    /// The entity currently under the mouse cursor in the 3D viewport, if any. Recomputed every
    /// frame this tab is active (unlike `selected`, which only changes on click) so the hovered
    /// model can blink.
    hovered: Option<Entity>,
    renaming: Option<Entity>,
    rename_buffer: String,
    /// This tab's own viewport camera controller — kept per-tab (rather than shared) so switching
    /// tabs doesn't disturb whatever framing you last left each scene at.
    fly_camera: FlyCamera,
    /// Whether this tab has edits not yet written to disk — set whenever the UI mutates its
    /// `world`/`entity_assets` (see `ui::UiRequests::edited`), cleared by a successful Save/
    /// Export/tab-close autosave. Surfaced as a `*` on its tab (see `ui::draw_scene_tabs`) and
    /// drives the pre-Play "unsaved changes" prompt (see `EditorScene::play_confirmation`):
    /// `scene.change(...)` reads a scene's `.ron` straight off disk (see
    /// `libdqg::scripting`'s `WorldCommand::ChangeScene`), so a dirty *other* open tab would
    /// otherwise be silently stale to a script that jumps to it mid-Play, even though its real,
    /// unsaved contents are fully visible on screen right now.
    dirty: bool,
}

impl OpenScene {
    fn new(path: PathBuf, world: World, entity_assets: HashMap<Entity, RenderableAsset>) -> Self {
        Self {
            path,
            world,
            entity_assets,
            selected: None,
            hovered: None,
            renaming: None,
            rename_buffer: String::new(),
            fly_camera: FlyCamera::new(6.0),
            dirty: false,
        }
    }
}

/// The starting viewport camera every fresh [`World`] gets — extracted so [`EditorScene::opening`]
/// and [`EditorScene::start_load`] (which each build a brand new `World`) don't duplicate it.
fn default_camera() -> Camera {
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
    camera
}

pub struct EditorScene {
    /// Every scene currently open for editing — always has at least one entry (a blank
    /// placeholder before any project is opened, same as the editor's old always-present single
    /// `World`), so [`Self::active_scene`] is always a valid index into it.
    open_scenes: Vec<OpenScene>,
    active_scene: usize,
    /// Elapsed time accumulator driving the hover blink, shared across tabs since it's a plain
    /// animation clock, not per-scene state.
    hover_blink_time: f32,
    project: Option<Project>,
    last_mouse_pos: (f32, f32),
    recent: RecentProjects,
    load: Option<LoadState>,
    egui: EguiLayer,
    assets_expanded: bool,
    /// Cached texture thumbnails for the Assets panel, keyed by project-relative path — global
    /// rather than per-tab since it's a decode cache for the project's `assets/` folder, not
    /// scene-specific state; cleared only when a different project is opened.
    texture_previews: HashMap<PathBuf, egui::TextureHandle>,
    mode: EditorMode,
    /// Recent script compile/runtime errors and when each was recorded, for the overlay in
    /// [`ui::draw_script_error_overlay`] — pruned by [`SCRIPT_ERROR_DISPLAY`] each frame and
    /// cleared outright on Stop.
    script_errors: Vec<(String, Instant)>,
    /// The outcome of the last Export (see `ui::draw_export_status_overlay`), and when it was
    /// recorded — aged out after [`EXPORT_STATUS_DISPLAY`], same shape as `script_errors` but for
    /// one message instead of a list.
    export_status: Option<(String, Instant)>,
    /// Editor-wide preferences (currently just the external editor "Open Script" launches),
    /// persisted next to the executable like [`RecentProjects`].
    settings: EditorSettings,
    /// The script currently being renamed inline in the Assets panel, if any — mirrors
    /// `renaming`/`rename_buffer` above, just keyed by a script's project-relative path instead
    /// of an [`Entity`] since scripts aren't ECS entities.
    renaming_script: Option<PathBuf>,
    script_rename_buffer: String,
    /// Mirrors `renaming_script`/`script_rename_buffer`, for a scene tile in the Assets panel's
    /// Scenes section.
    renaming_scene: Option<PathBuf>,
    scene_rename_buffer: String,
    /// `Some(dirty tab paths)` while the pre-Play "unsaved changes" dialog (see
    /// [`draw_play_confirmation`]) is up, blocking the rest of `update` until resolved — set when
    /// Play is requested while any tab is dirty, cleared once the user picks Save & Play, Play
    /// Without Saving, or Cancel.
    play_confirmation: Option<Vec<PathBuf>>,
}

impl EditorScene {
    /// Builds a scene that will create or open `action`'s project root on its first `update`.
    pub fn opening(action: PendingAction) -> Self {
        Self {
            open_scenes: vec![OpenScene::new(PathBuf::new(), World::new(default_camera()), HashMap::new())],
            active_scene: 0,
            hover_blink_time: 0.0,
            project: None,
            last_mouse_pos: (0.0, 0.0),
            recent: RecentProjects::load(),
            load: Some(LoadState::Pending(action)),
            egui: EguiLayer::new(),
            assets_expanded: true,
            texture_previews: HashMap::new(),
            mode: EditorMode::Edit,
            script_errors: Vec::new(),
            export_status: None,
            settings: EditorSettings::load(),
            renaming_script: None,
            script_rename_buffer: String::new(),
            renaming_scene: None,
            scene_rename_buffer: String::new(),
            play_confirmation: None,
        }
    }

    /// Queues `action` to start on the next `update`.
    fn queue_action(&mut self, action: PendingAction) {
        self.load = Some(LoadState::Pending(action));
    }

    /// Resolves `action` into an open [`Project`] (fast: just filesystem/manifest/scene-file
    /// work, no asset decoding), replacing every currently open tab with the project's start
    /// scene and queuing its entities — if any — to be loaded incrementally afterward. Records
    /// the project in the recent-projects list. A different project's tabs are meaningless once
    /// the project changes, so unlike [`Self::switch_or_open_scene`] there's nothing to preserve
    /// here.
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

        let scene_path = project.manifest.start_scene.clone();
        let entities: VecDeque<EntityRecord> = if create {
            VecDeque::new()
        } else {
            match project.load_scene(&scene_path) {
                Ok(scene_file) => scene_file.entities.into(),
                Err(e) => {
                    // Same "don't touch anything" abort as `switch_or_open_scene`'s own
                    // load-failure path: falling through to an empty scene here would let a
                    // later Save silently overwrite whatever's actually in this corrupt/
                    // unreadable file, and (for an already-open project) would also wipe out
                    // every existing tab for no reason.
                    eprintln!("Failed to load scene: {e}");
                    return;
                }
            }
        };

        self.recent.add(root);
        let _ = self.recent.save();

        // A live ScriptRuntime holds Entity handles into an old World; opening a different
        // project invalidates every current tab, so stop Play first.
        self.mode = EditorMode::Edit;
        self.script_errors.clear();
        self.texture_previews.clear();

        let world = World::new(default_camera());
        if entities.is_empty() {
            self.open_scenes = vec![OpenScene::new(scene_path, world, HashMap::new())];
            self.active_scene = 0;
            self.project = Some(project);
        } else {
            let total = entities.len();
            self.load = Some(LoadState::LoadingProject { project, path: scene_path, world, entity_assets: HashMap::new(), remaining: entities, total });
        }
    }

    /// Switches to `path` (project-relative) within the currently open project: if it's already
    /// open in a tab, just activates that tab (no reload — the whole point of keeping scenes
    /// loaded). Otherwise loads it as a brand new tab, appended once loaded, leaving every other
    /// tab untouched. A no-op if no project is open, or if `path` fails to load (reported, no tab
    /// opened, so a corrupt scene file doesn't risk getting silently overwritten by a later Save).
    /// Stops Play first if it's running — a live `ScriptRuntime` holds `Entity` handles into
    /// whichever tab was active when Play started, and switching away from it would leave those
    /// pointing at nothing.
    fn switch_or_open_scene(&mut self, path: PathBuf) {
        if matches!(self.mode, EditorMode::Playing { .. }) {
            self.stop_play();
        }

        if let Some(index) = self.open_scenes.iter().position(|scene| scene.path == path) {
            self.active_scene = index;
            return;
        }

        let Some(project) = self.project.as_ref() else { return };
        let scene_file = match project.load_scene(&path) {
            Ok(scene_file) => scene_file,
            Err(e) => {
                eprintln!("Failed to load scene {}: {e}", path.display());
                return;
            }
        };

        let entities: VecDeque<EntityRecord> = scene_file.entities.into();
        let world = World::new(default_camera());
        if entities.is_empty() {
            self.open_scenes.push(OpenScene::new(path, world, HashMap::new()));
            self.active_scene = self.open_scenes.len() - 1;
        } else {
            let total = entities.len();
            self.load = Some(LoadState::LoadingScene { path, world, entity_assets: HashMap::new(), remaining: entities, total });
        }
    }

    /// Closes the tab for `path`, autosaving it first — a no-op if it's the only tab open (a
    /// project always keeps at least one scene open for editing) or if `path` isn't open. Stops
    /// Play first if the tab being closed is the one Play is currently targeting.
    fn close_scene_tab(&mut self, path: PathBuf) {
        if self.open_scenes.len() <= 1 {
            return;
        }
        let Some(index) = self.open_scenes.iter().position(|scene| scene.path == path) else { return };

        if index == self.active_scene && matches!(self.mode, EditorMode::Playing { .. }) {
            self.stop_play();
        }

        if let Some(project) = self.project.as_ref() {
            let scene = &self.open_scenes[index];
            if let Err(e) = project.save_scene(&scene.path, &scene.world, &scene.entity_assets) {
                // Keep the tab open on a failed autosave — removing it anyway would discard the
                // only copy of whatever wasn't saved, contradicting this method's own "autosaving
                // it first" contract. Lets the user retry the close (or just Save) once whatever
                // caused the failure (disk full, permissions, ...) is resolved.
                eprintln!("Failed to save {}: {e}", scene.path.display());
                return;
            }
        }

        self.open_scenes.remove(index);
        if index < self.active_scene {
            self.active_scene -= 1;
        } else if self.active_scene >= self.open_scenes.len() {
            self.active_scene = self.open_scenes.len() - 1;
        }
    }

    /// Renames the scene file at `old_path` (project-relative) to `new_stem`, keeping its
    /// extension, then updates `ProjectManifest.start_scene` (if it named the renamed scene) and
    /// any open tab's identity to match. Refuses (and reports, rather than silently overwriting)
    /// if something is already using the target name. Doesn't fix up `scene.change("...")` calls
    /// inside script source — see `ui::draw_scene_tile`'s doc comment for why that's not possible
    /// the way [`Self::rename_script`] fixes up `ScriptAttachment`s.
    fn rename_scene(&mut self, old_path: PathBuf, new_stem: String) {
        let new_stem = new_stem.trim();
        if new_stem.is_empty() {
            return;
        }
        let extension = old_path.extension().map(|ext| ext.to_string_lossy().into_owned()).unwrap_or_default();
        let new_path = old_path.with_file_name(format!("{new_stem}.{extension}"));
        if new_path == old_path {
            return;
        }

        let Some(project) = self.project.as_mut() else { return };
        let old_absolute = project.root.join(&old_path);
        let new_absolute = project.root.join(&new_path);
        if new_absolute.exists() {
            eprintln!("Failed to rename scene: {} already exists", new_path.display());
            return;
        }
        if let Err(e) = std::fs::rename(&old_absolute, &new_absolute) {
            eprintln!("Failed to rename scene: {e}");
            return;
        }

        if project.manifest.start_scene == old_path {
            if let Err(e) = project.set_start_scene(new_path.clone()) {
                eprintln!("Failed to update start scene after rename: {e}");
            }
        }

        for scene in &mut self.open_scenes {
            if scene.path == old_path {
                scene.path = new_path.clone();
            }
        }
    }

    /// Renames the script file at `old_path` (project-relative) to `new_stem`, keeping its
    /// extension, then fixes up every [`libdqg::scripting::ScriptAttachment`] that referenced the
    /// old path — in every open tab's live `World` *and* every scene file on disk that isn't
    /// currently open (loaded, patched, and saved back only if it actually referenced the old
    /// path). A script can be attached to entities in any scene in the project, not just whichever
    /// one happens to be the active tab, so both are needed — fixing up only the active `World`
    /// (as an earlier, single-scene version of this did) would silently leave every other scene's
    /// reference pointing at a file that no longer exists. Refuses (and reports, rather than
    /// silently overwriting) if something is already using the target name.
    ///
    /// Copies rather than renames the file up front, deleting the original only once every scene
    /// has been migrated (or confirmed not to reference it) — a scene file that fails to load or
    /// save partway through this (corrupt on disk, a failed write, ...) is rare but not
    /// impossible, and with a plain rename that failure would leave that scene's attachment
    /// pointing at a file that's already gone. Copying first means both names stay valid on disk
    /// for the duration, so a partial failure just leaves some scenes referencing the old name
    /// and some the new one — both of which still resolve — rather than a dangling reference.
    fn rename_script(&mut self, old_path: PathBuf, new_stem: String) {
        let new_stem = new_stem.trim();
        if new_stem.is_empty() {
            return;
        }
        let extension = old_path.extension().map(|ext| ext.to_string_lossy().into_owned()).unwrap_or_default();
        let new_path = old_path.with_file_name(format!("{new_stem}.{extension}"));
        if new_path == old_path {
            return;
        }

        let old_absolute = {
            let Some(project) = self.project.as_ref() else { return };
            let old_absolute = project.root.join(&old_path);
            let new_absolute = project.root.join(&new_path);
            if new_absolute.exists() {
                eprintln!("Failed to rename script: {} already exists", new_path.display());
                return;
            }
            if let Err(e) = std::fs::copy(&old_absolute, &new_absolute) {
                eprintln!("Failed to rename script: {e}");
                return;
            }
            old_absolute
        };

        let mut all_migrated = true;

        // Every open tab's live World — marking a tab dirty only if it actually referenced the
        // renamed script, so this doesn't spuriously flag unrelated tabs as unsaved. In-memory
        // updates can't fail the way a disk load/save below can, so these always count as
        // migrated (the tab's own Save/autosave path is what persists this afterward).
        let open_paths: HashSet<PathBuf> = self.open_scenes.iter().map(|scene| scene.path.clone()).collect();
        for scene in self.open_scenes.iter_mut() {
            let mut changed = false;
            for (_, list) in scene.world.scripts.iter_mut() {
                for attachment in list.0.iter_mut() {
                    if attachment.path == old_path {
                        attachment.path = new_path.clone();
                        changed = true;
                    }
                }
            }
            if changed {
                scene.dirty = true;
            }
        }

        // Every other scene file on disk — anything already covered above via an open tab is
        // skipped here, since that in-memory copy is the current source of truth until it's saved.
        if let Some(project) = self.project.as_ref() {
            for scene_path in project.list_scenes() {
                if open_paths.contains(&scene_path) {
                    continue;
                }
                let mut scene_file = match project.load_scene(&scene_path) {
                    Ok(scene_file) => scene_file,
                    Err(e) => {
                        eprintln!("Failed to check {} for script references: {e}", scene_path.display());
                        all_migrated = false;
                        continue;
                    }
                };
                let mut changed = false;
                for record in scene_file.entities.iter_mut() {
                    for attachment in record.scripts.iter_mut() {
                        if attachment.path == old_path {
                            attachment.path = new_path.clone();
                            changed = true;
                        }
                    }
                }
                if changed {
                    if let Err(e) = scene_file.save(&project.root.join(&scene_path)) {
                        eprintln!("Failed to update script reference in {}: {e}", scene_path.display());
                        all_migrated = false;
                    }
                }
            }
        }

        if all_migrated {
            if let Err(e) = std::fs::remove_file(&old_absolute) {
                eprintln!("Renamed script to {}, but failed to remove the old file: {e}", new_path.display());
            }
        } else {
            eprintln!(
                "Some scenes still reference {} — left it alongside {} on disk until they're fixed up \
                 (open the affected scene and re-save, or reattach {})",
                old_path.display(),
                new_path.display(),
                new_path.display()
            );
        }
    }

    /// Snapshots the active tab's whole [`World`] and starts every enabled script attachment (in
    /// order) on every entity that has one, then switches to [`EditorMode::Playing`]. A no-op if
    /// there's no open project (the Play button is disabled in that case anyway — see
    /// `ui::draw_menu_bar`).
    fn start_play(&mut self, renderer: &Renderer) {
        let Some(project) = self.project.as_ref() else { return };
        let scene = &mut self.open_scenes[self.active_scene];

        let world_snapshot = scene.world.clone();
        let mut runtime = ScriptRuntime::new(project.root.clone());
        // Without this, `world.find(...)` inside an `on_start` hook would see nothing but a
        // default-empty snapshot (only populated per-frame from here on, right before
        // `update_entity`) and always return `()` — `on_start` gets the same start-of-Play
        // snapshot guarantee `on_update` already has for cross-entity reads.
        runtime.begin_frame(&scene.world);

        for error in runtime.start_all_scripts(&mut scene.world, &project.root, Some(renderer)) {
            self.script_errors.push((format_script_error(&error), Instant::now()));
        }

        self.mode = EditorMode::Playing { world_snapshot, runtime };
    }

    /// Restores the whole [`World`] [`start_play`](Self::start_play) snapshotted (into whichever
    /// tab was active when Play started) and drops the [`ScriptRuntime`] (and with it every
    /// script's persistent state), switching back to [`EditorMode::Edit`].
    fn stop_play(&mut self) {
        if let EditorMode::Playing { world_snapshot, .. } = std::mem::replace(&mut self.mode, EditorMode::Edit) {
            self.open_scenes[self.active_scene].world = world_snapshot;
        }
        self.script_errors.clear();
    }

    /// Handles a Play click: if any tab is dirty, holds off starting Play and instead raises the
    /// confirmation dialog (see [`Self::play_confirmation`]) — `scene.change(...)` reads straight
    /// off disk, so a script that jumps to a dirty *other* tab during this session would otherwise
    /// silently see whatever was last saved there, not what's on screen right now. Starts Play
    /// immediately, with no prompt, if nothing is dirty.
    fn request_play(&mut self, renderer: &Renderer) {
        let dirty_paths: Vec<PathBuf> = self.open_scenes.iter().filter(|scene| scene.dirty).map(|scene| scene.path.clone()).collect();
        if dirty_paths.is_empty() {
            self.start_play(renderer);
        } else {
            self.play_confirmation = Some(dirty_paths);
        }
    }

    /// Saves every dirty open tab (not just the active one) — the "Save & Play" choice in the
    /// pre-Play confirmation dialog, and also what a successful Export does before packaging (see
    /// `Scene::update`'s `export_project` handling) — both need every tab's on-disk copy current,
    /// not just whichever one is active.
    fn save_all_dirty(&mut self) {
        let Some(project) = self.project.as_ref() else { return };
        for scene in self.open_scenes.iter_mut() {
            if !scene.dirty {
                continue;
            }
            match project.save_scene(&scene.path, &scene.world, &scene.entity_assets) {
                Ok(()) => scene.dirty = false,
                Err(e) => eprintln!("Failed to save {}: {e}", scene.path.display()),
            }
        }
    }
}

/// What the user chose in the pre-Play "unsaved changes" dialog (see [`draw_play_confirmation`]).
enum PlayConfirmAction {
    SaveAndPlay,
    PlayWithoutSaving,
    Cancel,
}

/// Shown instead of the normal editor UI while [`EditorScene::play_confirmation`] is set — Play
/// was requested while one or more tabs have unsaved edits. Lists which scenes are dirty so the
/// choice is informed, then lets the user save everything first, play anyway (accepting that a
/// `scene.change(...)` to one of these would load its last-saved, not current, contents), or
/// cancel.
fn draw_play_confirmation(ui: &mut egui::Ui, dirty_paths: &[PathBuf]) -> Option<PlayConfirmAction> {
    let mut action = None;
    egui::CentralPanel::default().show(ui, |ui| {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label("Unsaved changes in:");
                for path in dirty_paths {
                    ui.label(path.display().to_string());
                }
                ui.add_space(8.0);
                ui.label("A script that switches to one of these during Play would load its last-saved contents, not what's shown here.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save & Play").clicked() {
                        action = Some(PlayConfirmAction::SaveAndPlay);
                    }
                    if ui.button("Play Without Saving").clicked() {
                        action = Some(PlayConfirmAction::PlayWithoutSaving);
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some(PlayConfirmAction::Cancel);
                    }
                });
            });
        });
    });
    action
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
                LoadState::LoadingProject { project, path, mut world, mut entity_assets, mut remaining, total } => {
                    let done = spawn_budgeted(&mut world, &mut entity_assets, &mut remaining, renderer, &project.root);
                    if done {
                        self.open_scenes = vec![OpenScene::new(path, world, entity_assets)];
                        self.active_scene = 0;
                        self.project = Some(project);
                    } else {
                        self.load = Some(LoadState::LoadingProject { project, path, world, entity_assets, remaining, total });
                    }
                }
                LoadState::LoadingScene { path, mut world, mut entity_assets, mut remaining, total } => {
                    if let Some(project) = self.project.as_ref() {
                        let done = spawn_budgeted(&mut world, &mut entity_assets, &mut remaining, renderer, &project.root);
                        if done {
                            self.open_scenes.push(OpenScene::new(path, world, entity_assets));
                            self.active_scene = self.open_scenes.len() - 1;
                        } else {
                            self.load = Some(LoadState::LoadingScene { path, world, entity_assets, remaining, total });
                        }
                    }
                }
            }

            if let Some(load_state) = &self.load {
                let (loaded, total) = match load_state {
                    LoadState::LoadingProject { remaining, total, .. } => (*total - remaining.len(), *total),
                    LoadState::LoadingScene { remaining, total, .. } => (*total - remaining.len(), *total),
                    LoadState::Pending(_) => (0, 0),
                };
                self.egui.run(renderer, |ui| draw_loading_screen(ui, loaded, total));
                return SceneTransition::None;
            }
        }

        if let Some(dirty_paths) = self.play_confirmation.clone() {
            let mut action = None;
            self.egui.run(renderer, |ui| action = draw_play_confirmation(ui, &dirty_paths));
            match action {
                Some(PlayConfirmAction::SaveAndPlay) => {
                    self.save_all_dirty();
                    self.play_confirmation = None;
                    self.start_play(renderer);
                }
                Some(PlayConfirmAction::PlayWithoutSaving) => {
                    self.play_confirmation = None;
                    self.start_play(renderer);
                }
                Some(PlayConfirmAction::Cancel) => {
                    self.play_confirmation = None;
                }
                None => {}
            }
            // Always yield the frame here, whether or not the dialog was just resolved — an
            // `egui::run` call was already spent drawing it above (only one is allowed per frame,
            // same constraint the loading screen has), so the normal UI can't also render this
            // same frame. One frame's delay picking up Play/Edit mode is imperceptible.
            return SceneTransition::None;
        }

        let mouse_pos = mouse_state.position();
        let mouse_delta = (mouse_pos.0 - self.last_mouse_pos.0, mouse_pos.1 - self.last_mouse_pos.1);
        self.last_mouse_pos = mouse_pos;

        self.script_errors.retain(|(_, at)| at.elapsed() < SCRIPT_ERROR_DISPLAY);
        if self.export_status.as_ref().is_some_and(|(_, at)| at.elapsed() >= EXPORT_STATUS_DISPLAY) {
            self.export_status = None;
        }
        let is_playing = matches!(self.mode, EditorMode::Playing { .. });
        let script_error_messages: Vec<String> = self.script_errors.iter().map(|(message, _)| message.clone()).collect();
        let export_status_message = self.export_status.as_ref().map(|(message, _)| message.as_str());
        let open_scene_tabs: Vec<(PathBuf, bool)> = self.open_scenes.iter().map(|scene| (scene.path.clone(), scene.dirty)).collect();
        let active_scene = self.active_scene;

        let active_index = self.active_scene;
        let (world, selected, renaming, rename_buffer, entity_assets, current_scene, current_scene_dirty) = {
            let scene = &mut self.open_scenes[active_index];
            (&mut scene.world, &mut scene.selected, &mut scene.renaming, &mut scene.rename_buffer, &mut scene.entity_assets, scene.path.clone(), scene.dirty)
        };
        let project = self.project.as_ref();
        let assets_expanded = &mut self.assets_expanded;
        let texture_previews = &mut self.texture_previews;
        let settings = &mut self.settings;
        let renaming_script = &mut self.renaming_script;
        let script_rename_buffer = &mut self.script_rename_buffer;
        let renaming_scene = &mut self.renaming_scene;
        let scene_rename_buffer = &mut self.scene_rename_buffer;
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
                renaming_scene,
                scene_rename_buffer,
                export_status_message,
                &current_scene,
                current_scene_dirty,
                &open_scene_tabs,
                active_scene,
                &mut requests,
            );
        });

        if requests.edited {
            self.open_scenes[active_index].dirty = true;
        }

        if requests.save_scene {
            if let Some(project) = self.project.as_ref() {
                let scene = &self.open_scenes[active_index];
                match project.save_scene(&scene.path, &scene.world, &scene.entity_assets) {
                    Ok(()) => self.open_scenes[active_index].dirty = false,
                    Err(e) => eprintln!("Failed to save project: {e}"),
                }
            }
        }

        if let Some(root) = requests.new_project.take() {
            self.queue_action(PendingAction::New(root));
        }

        if let Some(output_dir) = requests.export_project.take() {
            if self.project.is_some() {
                // Export packages the scene files off disk, not any tab's `World` directly — save
                // every dirty tab first (not just the active one) so unsaved edits anywhere
                // actually make it into the exported game.
                self.save_all_dirty();
            }
            if let Some(project) = self.project.as_ref() {
                let result = crate::export::export_project(project, &output_dir);
                let message = match &result {
                    Ok(()) => format!("Exported to {}", output_dir.display()),
                    Err(e) => format!("Export failed: {e}"),
                };
                if let Err(e) = &result {
                    eprintln!("Export failed: {e}");
                }
                self.export_status = Some((message, Instant::now()));
            }
        }

        if let Some(root) = requests.open_project.take() {
            self.queue_action(PendingAction::Open(root));
        }

        if let Some(path) = requests.switch_scene.take() {
            self.switch_or_open_scene(path);
        }

        if let Some(path) = requests.close_scene_tab.take() {
            self.close_scene_tab(path);
        }

        if let Some((old_path, new_stem)) = requests.rename_scene.take() {
            self.rename_scene(old_path, new_stem);
        }

        if let Some((old_path, new_stem)) = requests.rename_script.take() {
            self.rename_script(old_path, new_stem);
        }

        if let Some(path) = requests.set_start_scene.take() {
            if let Some(project) = self.project.as_mut() {
                if let Err(e) = project.set_start_scene(path) {
                    eprintln!("Failed to set start scene: {e}");
                }
            }
        }

        if requests.toggle_play {
            if is_playing {
                self.stop_play();
            } else {
                self.request_play(renderer);
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
                match probe_asset.load(renderer, &project.root) {
                    Ok(renderable) => {
                        // Re-derive the sprite's width/height from the loaded texture's native
                        // size rather than trusting the placeholder above.
                        let asset = match &renderable {
                            Renderable::Sprite(sprite) => {
                                RenderableAsset::Sprite { texture_path: path, width: sprite.width, height: sprite.height }
                            }
                            Renderable::Model(_) => RenderableAsset::Model { model_path: path },
                        };
                        let scene = &mut self.open_scenes[self.active_scene];
                        scene.world.set_renderable(entity, renderable);
                        scene.entity_assets.insert(entity, asset);
                        scene.dirty = true;
                    }
                    Err(e) => eprintln!("Failed to attach renderable: {e}"),
                }
            }
        }

        self.hover_blink_time += delta_time;

        let active_index = self.active_scene;
        match &mut self.mode {
            EditorMode::Edit => {
                if !self.egui.wants_pointer_input() {
                    let scene = &mut self.open_scenes[active_index];
                    scene.fly_camera.update(delta_time, input_state, mouse_state, &mut scene.world.camera, mouse_delta);

                    let size = renderer.size();
                    let ndc_x = (mouse_pos.0 / (size.width.max(1) as f32)) * 2.0 - 1.0;
                    let ndc_y = 1.0 - (mouse_pos.1 / (size.height.max(1) as f32)) * 2.0;
                    scene.hovered = picking::pick(&scene.world, &scene.world.camera, ndc_x, ndc_y);

                    if mouse_state.is_button_pressed(winit::event::MouseButton::Left) {
                        scene.selected = scene.hovered;
                    }
                } else {
                    self.open_scenes[active_index].hovered = None;
                }
            }
            EditorMode::Playing { runtime, .. } => {
                // Camera flying and viewport picking are edit-mode-only for v1 — playing just
                // runs scripts against the live World; see the implementation plan's
                // "Camera/picking fully disabled during Play" note.
                let scene = &mut self.open_scenes[active_index];
                scene.hovered = None;
                runtime.begin_frame(&scene.world);
                for entity in scene.world.iter_entities().collect::<Vec<_>>() {
                    for error in runtime.update_entity(&mut scene.world, entity, delta_time, input_state, mouse_state, mouse_delta, Some(&*renderer)) {
                        self.script_errors.push((format_script_error(&error), Instant::now()));
                    }
                }
            }
        }

        let scene = &mut self.open_scenes[active_index];
        scene.world.sync_transforms();
        let size = renderer.size();
        let aspect = size.width as f32 / (size.height.max(1) as f32);
        scene.world.camera.aspect = aspect;

        let push_camera = if is_playing {
            scene.world.active_camera(aspect).unwrap_or(scene.world.camera)
        } else {
            scene.world.camera
        };
        *renderer.camera_mut() = push_camera;

        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        let blink_alpha = 0.15 + 0.35 * (self.hover_blink_time * 6.0).sin().abs();
        let hover_highlight = Color::new(1.0, 1.0, 1.0, blink_alpha as f64);
        let no_highlight = Color::new(1.0, 1.0, 1.0, 0.0);
        let is_edit_mode = matches!(self.mode, EditorMode::Edit);
        let scene = &mut self.open_scenes[self.active_scene];

        for (entity, renderable) in scene.world.renderables.iter_mut() {
            let highlight = if scene.hovered == Some(entity) { hover_highlight } else { no_highlight };
            let is_selected = scene.selected == Some(entity);

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

        if is_edit_mode {
            for (entity, component) in scene.world.cameras.iter() {
                let Some(transform) = scene.world.transforms.get(entity) else { continue };
                let color = if scene.selected == Some(entity) {
                    Color::new(0.3, 0.7, 1.0, 1.0)
                } else {
                    Color::new(0.6, 0.6, 0.6, 1.0)
                };
                draw_camera_gizmo(pass, &scene.world.camera, transform, component, color);
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
                ui.label(format!("Loading scene... ({loaded}/{total})"));
            });
        });
    });
}
