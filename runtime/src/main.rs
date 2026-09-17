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
use libdqg::scripting::{ScriptList, ScriptRuntime};
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

fn load_manifest(res_dir: &Path) -> GameManifest {
    fs::read_to_string(res_dir.join("game.ron"))
        .ok()
        .and_then(|text| ron::from_str(&text).ok())
        .unwrap_or_else(|| GameManifest { title: "libDQG Game".to_string(), width: 1280, height: 720 })
}

fn load_scene_file(res_dir: &Path) -> SceneFile {
    fs::read_to_string(res_dir.join("scene.ron"))
        .ok()
        .and_then(|text| ron::from_str(&text).ok())
        .unwrap_or_default()
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
    last_mouse_pos: (f32, f32),
}

impl RuntimeScene {
    fn new(res_dir: PathBuf) -> Self {
        Self { state: RuntimeState::Loading, res_dir, last_mouse_pos: (0.0, 0.0) }
    }

    /// Builds the `World` from `res/scene.ron` and starts every enabled script attachment —
    /// mirrors the editor's `EditorScene::continue_load` (entity spawn sequence) followed by
    /// `EditorScene::start_play` (script startup), but in one pass: there's no editor UI here
    /// that needs the load spread across frames.
    fn start(&mut self, renderer: &Renderer) {
        let scene_file = load_scene_file(&self.res_dir);

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
            let entity = world.spawn_empty(record.name, record.transform);
            if let Some(asset) = record.renderable {
                match asset.load(renderer, &self.res_dir) {
                    Ok(renderable) => world.set_renderable(entity, renderable),
                    Err(e) => eprintln!("Failed to load renderable: {e}"),
                }
            }
            if !record.scripts.is_empty() {
                world.scripts.insert(entity, ScriptList(record.scripts));
            }
            if let Some(component) = record.camera {
                world.set_camera(entity, component);
            }
        }

        let mut runtime = ScriptRuntime::new(self.res_dir.clone());
        runtime.begin_frame(&world);
        for entity in world.iter_entities().collect::<Vec<_>>() {
            let Some(list) = world.scripts.get(entity) else { continue };
            let starts: Vec<(usize, PathBuf)> = list
                .0
                .iter()
                .enumerate()
                .filter(|(_, attachment)| attachment.enabled)
                .map(|(index, attachment)| (index, self.res_dir.join(&attachment.path)))
                .collect();

            for (index, script_path) in starts {
                if let Err(e) = runtime.start_script(&world, entity, index, &script_path) {
                    eprintln!("Script error ({}): {}", e.script.display(), e.message);
                }
            }
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
        let mouse_delta = (mouse_pos.0 - self.last_mouse_pos.0, mouse_pos.1 - self.last_mouse_pos.1);
        self.last_mouse_pos = mouse_pos;

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
    let manifest = load_manifest(&res_dir);

    let mut game = GameBuilder::new(Box::new(RuntimeScene::new(res_dir)))
        .title(manifest.title)
        .size(manifest.width, manifest.height)
        .build();
    game.run();
}
