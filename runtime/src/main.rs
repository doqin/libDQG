//! The export template: a generic, project-agnostic standalone binary for a game exported from
//! the libDQG editor. Built once ahead of time and shipped alongside the editor (see
//! `editor::export`); at its own startup it reads a project's scene/scripts/assets from a `res/`
//! folder next to itself, so exporting a project is a pure file-copy operation with no
//! compilation needed.

// No console window on Windows for a shipped (release) game — but keep it in debug builds so
// `cargo run -p runtime`/a debug export still shows the `eprintln!` script/asset error output
// below.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use libdqg::camera::Camera;
use libdqg::game::GameBuilder;
use libdqg::glam;
use libdqg::input::{InputState, MouseState};
use libdqg::renderer::{DrawPass, Renderer};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::scene_file::{GameManifest, SceneFile};
use libdqg::scripting::ScriptRuntime;
use libdqg::world::{Renderable, World};

/// An exported game's data directory: `res/` next to its own executable, mirroring the
/// exe-relative convention `libdqg::util::resolve_resource_path`/`copy_res_to_output_dir`
/// already use for `demo`'s bundled assets.
fn res_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("res")))
        .unwrap_or_else(|| PathBuf::from("res"))
}

/// A missing or corrupt `game.ron`/scene file means the export itself is broken (the editor's
/// `export::export_project` always writes both) — surfaced as a hard startup failure rather than
/// silently falling back to defaults/an empty scene, which would ship a game that looks like it
/// works but is missing everything.
fn load_manifest(res_dir: &Path) -> anyhow::Result<GameManifest> {
    Ok(ron::from_str(&fs::read_to_string(res_dir.join("game.ron"))?)?)
}

/// Prints `message` (with `path` and the error) and exits the process — the "visible diagnostic"
/// [`load_manifest`]/[`SceneFile::load`] fail loudly with instead of a silent fallback.
fn fail_to_load(path: &Path, error: anyhow::Error) -> ! {
    eprintln!("Failed to load {}: {error}", path.display());
    std::process::exit(1);
}

/// Whether the exported scene has been loaded and its scripts started yet — deferred past the
/// first `update` because loading needs a live [`Renderer`] (for texture/model decoding), which
/// only exists once the window/GPU surface does (see `Scene::update`'s `renderer` doc comment and
/// `demo`'s own `=== LOADING ===` pattern).
enum RuntimeState {
    Loading,
    Running { world: World, runtime: ScriptRuntime },
}

struct RuntimeScene {
    state: RuntimeState,
    res_dir: PathBuf,
    /// Project-relative path (from `GameManifest::start_scene`) to the scene this game boots
    /// into.
    start_scene: PathBuf,
    /// `None` until the first frame's mouse position is known, so that first frame reports a
    /// zero delta instead of an artificial jump from the origin to wherever the cursor actually
    /// starts.
    last_mouse_pos: Option<(f32, f32)>,
}

impl RuntimeScene {
    fn new(res_dir: PathBuf, start_scene: PathBuf) -> Self {
        Self { state: RuntimeState::Loading, res_dir, start_scene, last_mouse_pos: None }
    }

    /// Builds the `World` from the start scene and starts every enabled script attachment —
    /// mirrors the editor's `EditorScene::continue_load` (entity spawn sequence) followed by
    /// `EditorScene::start_play` (script startup), but in one pass: there's no editor UI here
    /// that needs the load spread across frames. Uses the same `EntityRecord::spawn_into`/
    /// `ScriptRuntime::start_all_scripts` helpers a script-triggered `scene.change(...)` uses
    /// mid-game, so initial boot and a later scene change behave identically.
    fn start(&mut self, renderer: &Renderer) {
        let scene_path = self.res_dir.join(&self.start_scene);
        let scene_file = SceneFile::load(&scene_path).unwrap_or_else(|e| fail_to_load(&scene_path, e));

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
        let mut world = World::new(camera);

        for record in scene_file.entities {
            record.spawn_into(&mut world, Some(renderer), &self.res_dir);
        }

        let mut runtime = ScriptRuntime::new(self.res_dir.clone());
        runtime.begin_frame(&world);
        for error in runtime.start_all_scripts(&world, &self.res_dir) {
            eprintln!("Script error ({}): {}", error.script.display(), error.message);
        }

        self.state = RuntimeState::Running { world, runtime };
    }
}

impl Scene for RuntimeScene {
    fn update(
        &mut self,
        delta_time: f32,
        input_state: &InputState,
        mouse_state: &MouseState,
        renderer: Option<&mut Renderer>,
    ) -> SceneTransition {
        let Some(renderer) = renderer else { return SceneTransition::None };

        if matches!(self.state, RuntimeState::Loading) {
            self.start(renderer);
        }

        let mouse_pos = mouse_state.position();
        let mouse_delta = self
            .last_mouse_pos
            .map(|last| (mouse_pos.0 - last.0, mouse_pos.1 - last.1))
            .unwrap_or((0.0, 0.0));
        self.last_mouse_pos = Some(mouse_pos);

        let RuntimeState::Running { world, runtime } = &mut self.state else {
            return SceneTransition::None;
        };

        runtime.begin_frame(world);
        for entity in world.iter_entities().collect::<Vec<_>>() {
            for error in runtime.update_entity(world, entity, delta_time, input_state, mouse_state, mouse_delta, Some(&*renderer)) {
                eprintln!("Script error ({}): {}", error.script.display(), error.message);
            }
        }

        world.sync_transforms();
        let size = renderer.size();
        let aspect = size.width as f32 / (size.height.max(1) as f32);
        world.camera.aspect = aspect;
        *renderer.camera_mut() = world.active_camera(aspect).unwrap_or(world.camera);

        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        let RuntimeState::Running { world, .. } = &mut self.state else { return };

        for (_, renderable) in world.renderables.iter_mut() {
            match renderable {
                Renderable::Sprite(sprite) => sprite.draw_world(pass),
                Renderable::Model(model) => pass.draw_model(model),
            }
        }
    }
}

fn main() {
    let res_dir = res_dir();
    let manifest_path = res_dir.join("game.ron");
    let manifest = load_manifest(&res_dir).unwrap_or_else(|e| fail_to_load(&manifest_path, e));
    let start_scene = manifest.start_scene.clone();

    let mut game = GameBuilder::new(Box::new(RuntimeScene::new(res_dir, start_scene)))
        .title(manifest.title)
        .size(manifest.width, manifest.height)
        .build();
    game.run();
}
