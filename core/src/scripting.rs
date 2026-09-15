use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use glam::{Quat, Vec3};
use rhai::{CustomType, Engine, FnPtr, Scope, TypeBuilder, AST};
use serde::{Deserialize, Serialize};

use crate::ecs::Entity;
use crate::input::InputState;
use crate::types::KeyCode;
use crate::world::{Transform, World};

/// One script file attached to an entity. `path` is project-root-relative, the same convention
/// [`crate::world::Renderable`]'s serialized form uses for its own asset paths.
#[derive(Clone, Serialize, Deserialize)]
pub struct ScriptAttachment {
    pub path: PathBuf,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// The scripts attached to one entity, executed front-to-back. Doubles as both the ECS
/// component and the serialized form — unlike [`crate::world::Renderable`], a script has no
/// live GPU-backed counterpart that needs a separate runtime representation.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ScriptList(pub Vec<ScriptAttachment>);

/// A script's own entity, as seen from inside Rhai: `entity.x`/`.y`/`.z` (get/set),
/// `entity.translate(x, y, z)`, `entity.rotate(x, y, z)`, `entity.scale(x, y, z)`,
/// `entity.name()`. Deliberately minimal for v1 — no way to reach any entity but its own.
///
/// Registered on the [`Engine`] via `#[derive(CustomType)]` (`ScriptRuntime::build_engine`'s
/// `engine.build_type::<ScriptApi>()`) instead of a hand-written builder chain:
/// - A plain field with no `#[rhai_type(...)]` attribute (`x`/`y`/`z`) is automatically exposed
///   as a get/set property under its own field name — that's the common case, so extending the
///   API with another simple scriptable number/string is just "add a field."
/// - `#[rhai_type(skip)]` opts a field out of that auto-registration entirely — used here for
///   `rotation`/`scale`/`name`, which are script-visible only through the methods in
///   [`ScriptApi::register_extra`], not as raw gettable/settable fields.
/// - `register_extra` (wired up via `#[rhai_type(extra = Self::register_extra)]` below) is where
///   anything that isn't a 1:1 field property — methods, or a skipped field that still needs
///   custom get/set logic — gets registered, in one place instead of scattered through
///   `build_engine`.
#[derive(Clone, CustomType)]
#[rhai_type(name = "Entity", extra = Self::register_extra)]
struct ScriptApi {
    x: f64,
    y: f64,
    z: f64,
    /// Kept as a [`Quat`], synced in/out of [`Transform::rotation`] as a plain copy — never
    /// decomposed to/from Euler angles on every frame. An Euler round-trip through
    /// `to_euler`/`from_euler` each frame is lossy (the middle axis of an Euler triple is
    /// extracted via `asin`, whose range is capped at ±90°), which made continuous single-axis
    /// rotation visibly stall once it crossed 90° — a gimbal-lock artifact from re-deriving the
    /// angle every frame instead of just accumulating it. `rotate()` composes an incremental
    /// delta quaternion instead, mirroring [`crate::transform::Transformable`]'s own
    /// post-multiply/local-space rotation convention.
    #[rhai_type(skip)]
    rotation: Quat,
    #[rhai_type(skip)]
    scale: Vec3,
    #[rhai_type(skip)]
    name: String,
}

impl ScriptApi {
    /// Everything the field-level `#[rhai_type]` attributes on [`ScriptApi`] can't express as a
    /// plain property: the `translate`/`rotate`/`scale`/`name` methods.
    fn register_extra(builder: &mut TypeBuilder<Self>) {
        builder
            .with_fn("translate", |e: &mut Self, x: f64, y: f64, z: f64| {
                e.x += x;
                e.y += y;
                e.z += z;
            })
            .with_fn("rotate", |e: &mut Self, x: f64, y: f64, z: f64| {
                let delta = Quat::from_euler(glam::EulerRot::XYZ, x as f32, y as f32, z as f32);
                e.rotation = (e.rotation * delta).normalize();
            })
            .with_fn("scale", |e: &mut Self, x: f64, y: f64, z: f64| {
                e.scale *= Vec3::new(x as f32, y as f32, z as f32);
            })
            .with_fn("name", |e: &mut Self| e.name.clone());
    }

    /// Seeds a fresh instance from an entity's current name/[`Transform`] — used once by
    /// [`ScriptRuntime::start_script`].
    fn from_transform(transform: &Transform, name: String) -> Self {
        let mut api = Self { x: 0.0, y: 0.0, z: 0.0, rotation: Quat::IDENTITY, scale: Vec3::ONE, name };
        api.sync_from_transform(transform);
        api
    }

    /// Overwrites position/rotation/scale from `transform`, leaving `name` untouched — the one
    /// place [`ScriptRuntime::update_entity`] needs to sync the *whole* transform in before
    /// calling `on_update`, rather than three separate per-field copies.
    fn sync_from_transform(&mut self, transform: &Transform) {
        self.x = transform.position.x as f64;
        self.y = transform.position.y as f64;
        self.z = transform.position.z as f64;
        self.rotation = transform.rotation;
        self.scale = transform.scale;
    }

    /// The inverse of [`ScriptApi::sync_from_transform`] — writes position/rotation/scale back
    /// out after `on_update` runs.
    fn write_into(&self, transform: &mut Transform) {
        transform.position = Vec3::new(self.x as f32, self.y as f32, self.z as f32);
        transform.rotation = self.rotation;
        transform.scale = self.scale;
    }
}

/// A read-only keyboard snapshot passed as `on_update`'s second argument:
/// `let on_update = |dt, input| { if input.is_held("KeyW") { entity.translate(0.0, 0.0, -dt); } };`.
/// Key names match [`KeyCode`]'s own variant identifiers via [`KeyCode::from_name`] (its derived
/// `Debug` output prints the same strings) — e.g. `"KeyW"`, `"ArrowUp"`, `"Space"`,
/// `"ShiftLeft"`. An unrecognized name is simply never held/pressed rather than an error, so a
/// typo in a key name fails quietly instead of aborting the script.
///
/// Rebuilt fresh from [`InputState`] every [`ScriptRuntime::update_entity`] call rather than
/// synced in/out like [`ScriptApi`] — input is read-only from a script's perspective, so there's
/// nothing to write back.
#[derive(Clone, CustomType)]
#[rhai_type(name = "Input", extra = Self::register_extra)]
struct ScriptInput {
    #[rhai_type(skip)]
    held: HashSet<KeyCode>,
    #[rhai_type(skip)]
    pressed: HashSet<KeyCode>,
}

impl ScriptInput {
    fn from_state(input: &InputState) -> Self {
        Self { held: input.keys_held().collect(), pressed: input.keys_pressed().collect() }
    }

    fn register_extra(builder: &mut TypeBuilder<Self>) {
        builder
            .with_fn("is_held", |i: &mut Self, name: &str| {
                KeyCode::from_name(name).is_some_and(|key| i.held.contains(&key))
            })
            .with_fn("is_pressed", |i: &mut Self, name: &str| {
                KeyCode::from_name(name).is_some_and(|key| i.pressed.contains(&key))
            });
    }
}

/// One error from compiling, starting, or running a script — never fatal to the editor, just
/// something to report and move past (see [`ScriptRuntime::update_entity`]).
#[derive(Debug)]
pub struct ScriptError {
    pub entity: Entity,
    pub script: PathBuf,
    pub message: String,
}

/// How many consecutive frames a script instance is allowed to error in [`ScriptRuntime::update_entity`]
/// before it disables itself (in the live [`ScriptList`], not on disk) so a broken script doesn't
/// spam the error overlay forever.
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

struct ScriptInstance {
    ast: Rc<AST>,
    scope: Scope<'static>,
    /// The script's `on_update` closure, if it defined one — captured once at start, but since
    /// Rhai closures share their captured `Scope` variables by reference (see
    /// `scripting::closure_state_spike`), calling this later still sees/mutates the same
    /// persistent state as `scope`.
    on_update: Option<FnPtr>,
    consecutive_failures: u32,
}

/// Compiles and caches one [`AST`] per unique script path (shared across every entity that
/// attaches the same script) and owns each `(entity, attachment index)`'s running Rhai `Scope`.
/// Created when the editor's Play mode starts and dropped when it stops — nothing here is meant
/// to survive a Stop; see the implementation plan's "Play/Stop snapshot" note for why script
/// state resetting on Stop is load-bearing, not just convenient.
pub struct ScriptRuntime {
    engine: Engine,
    compiled: HashMap<PathBuf, Rc<AST>>,
    instances: HashMap<(Entity, usize), ScriptInstance>,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        Self { engine: Self::build_engine(), compiled: HashMap::new(), instances: HashMap::new() }
    }

    fn build_engine() -> Engine {
        let mut engine = Engine::new();
        engine.build_type::<ScriptApi>();
        engine.build_type::<ScriptInput>();
        engine
    }

    /// Compiles (or reuses the cached [`AST`] for) the script at `script_path`, then starts a
    /// fresh instance of it for `(entity, index)`: runs the script body once — which defines its
    /// top-level locals and `on_start`/`on_update` closures — and calls `on_start()` if the
    /// script defined one. `world` supplies the entity's starting position/name so `on_start`
    /// sees real data rather than a zeroed placeholder.
    ///
    /// A compile/read/body error is fatal — no instance is created and `Err` is returned. An
    /// `on_start` error is not: the instance is still created (so `on_update` still runs on
    /// later frames) and the error is only reported, not propagated as a failure to start.
    pub fn start_script(&mut self, world: &World, entity: Entity, index: usize, script_path: &Path) -> Result<(), ScriptError> {
        let err = |message: String| ScriptError { entity, script: script_path.to_path_buf(), message };

        let ast = match self.compiled.get(script_path) {
            Some(ast) => ast.clone(),
            None => {
                let source = fs::read_to_string(script_path).map_err(|e| err(format!("failed to read script: {e}")))?;
                let ast = Rc::new(self.engine.compile(&source).map_err(|e| err(format!("compile error: {e}")))?);
                self.compiled.insert(script_path.to_path_buf(), ast.clone());
                ast
            }
        };

        let mut scope = Scope::new();
        let transform = world.transforms.get(entity).copied().unwrap_or_default();
        let name = world.names.get(entity).map(|n| n.0.clone()).unwrap_or_default();
        scope.push("entity", ScriptApi::from_transform(&transform, name));

        self.engine.run_ast_with_scope(&mut scope, &ast).map_err(|e| err(format!("script error: {e}")))?;

        let on_update = scope.get_value::<FnPtr>("on_update");
        let on_start = scope.get_value::<FnPtr>("on_start");

        let start_error = on_start.as_ref().and_then(|on_start| {
            on_start.call::<rhai::Dynamic>(&self.engine, &ast, ()).err().map(|e| err(format!("on_start error: {e}")))
        });

        self.instances.insert((entity, index), ScriptInstance { ast, scope, on_update, consecutive_failures: 0 });

        match start_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Calls every started, enabled attachment's `on_update(dt, input)` closure for `entity`, in
    /// attachment order (disabled attachments — including ones this call itself just disabled
    /// after too many failures — are skipped), syncing the entity's [`crate::world::Transform`]
    /// in beforehand and writing the result back out after. One instance erroring doesn't stop
    /// the others; every error encountered is returned rather than the first one short-circuiting.
    pub fn update_entity(&mut self, world: &mut World, entity: Entity, dt: f32, input: &InputState) -> Vec<ScriptError> {
        let mut errors = Vec::new();
        let Some(attachments) = world.scripts.get(entity).cloned() else { return errors };
        let input = ScriptInput::from_state(input);

        for (index, attachment) in attachments.0.iter().enumerate() {
            if !attachment.enabled {
                continue;
            }
            let Some(instance) = self.instances.get_mut(&(entity, index)) else { continue };
            let Some(on_update) = instance.on_update.clone() else { continue };

            // Sync the entity's transform into the script's API in one shot.
            if let Some(transform) = world.transforms.get(entity).copied() {
                if let Some(mut api) = scope_entity_mut(&mut instance.scope) {
                    api.sync_from_transform(&transform);
                }
            }

            // Call the script's `on_update(dt, input)` closure, which may mutate its captured `Scope` variables
            match on_update.call::<rhai::Dynamic>(&self.engine, &instance.ast, (dt as f64, input.clone())) {
                Ok(_) => {
                    instance.consecutive_failures = 0;
                    if let Some(api) = scope_entity_mut(&mut instance.scope) {
                        if let Some(transform) = world.transforms.get_mut(entity) {
                            api.write_into(transform);
                        }
                    }
                }
                Err(e) => {
                    instance.consecutive_failures += 1;
                    errors.push(ScriptError { entity, script: attachment.path.clone(), message: e.to_string() });

                    if instance.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        instance.consecutive_failures = 0;
                        if let Some(list) = world.scripts.get_mut(entity) {
                            if let Some(attachment) = list.0.get_mut(index) {
                                attachment.enabled = false;
                            }
                        }
                    }
                }
            }
        }

        errors
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrows the `entity` variable out of a script instance's `Scope` as `&mut ScriptApi`, working
/// whether or not it's been promoted to a shared value by closure capture (see
/// `scripting::closure_state_spike`).
fn scope_entity_mut<'a>(scope: &'a mut Scope<'static>) -> Option<rhai::DynamicWriteLock<'a, ScriptApi>> {
    scope.get_mut("entity")?.write_lock::<ScriptApi>()
}

#[cfg(test)]
mod closure_state_spike {
    //! Throwaway spike (see the implementation plan's "persistent script-local state" risk):
    //! confirms that a Rhai closure assigned to a `let` variable captures outer `Scope`
    //! variables *by reference*, so both a plain numeric counter and a custom-type "entity"
    //! object mutated inside the closure stay visible/persistent across repeated calls. If this
    //! stops holding on a future `rhai` upgrade, `ScriptRuntime` needs to fall back to plain
    //! `fn on_update(dt)` with state smuggled through the entity's own Transform instead.
    use rhai::{Engine, FnPtr, Scope};

    #[derive(Clone)]
    struct SpikeEntity {
        x: f64,
    }

    #[test]
    fn closure_captures_scope_variables_by_reference() {
        let mut engine = Engine::new();
        engine
            .register_type_with_name::<SpikeEntity>("Entity")
            .register_get_set("x", |e: &mut SpikeEntity| e.x, |e: &mut SpikeEntity, v: f64| e.x = v);

        let mut scope = Scope::new();
        scope.push("entity", SpikeEntity { x: 0.0 });

        let ast = engine
            .compile(
                r#"
                let counter = 0;
                let on_update = |dt| {
                    counter += 1;
                    entity.x += dt;
                    counter
                };
                "#,
            )
            .expect("script should compile");

        engine.run_ast_with_scope(&mut scope, &ast).expect("script body should run");

        let on_update: FnPtr = scope.get_value("on_update").expect("on_update closure should be in scope");

        let first: i64 = on_update.call(&engine, &ast, (1.5_f64,)).expect("first call should succeed");
        let second: i64 = on_update.call(&engine, &ast, (1.5_f64,)).expect("second call should succeed");

        assert_eq!(first, 1, "counter should persist across calls (closure captures by reference)");
        assert_eq!(second, 2);

        let entity: SpikeEntity = scope.get_value("entity").expect("entity should still be in scope");
        assert_eq!(entity.x, 3.0, "mutations inside the closure should be visible via the original Scope variable");
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;
    use crate::camera::Camera;
    use crate::world::Transform;

    fn test_camera() -> Camera {
        Camera { eye: Vec3::ZERO, target: Vec3::Z, up: Vec3::Y, aspect: 1.0, fov: 45.0, znear: 0.1, zfar: 100.0 }
    }

    /// An [`InputState`] with nothing held/pressed, for tests that don't care about input.
    fn no_input() -> InputState {
        InputState::for_test([], [])
    }

    /// Writes a `.rhai` source string to a fresh temp file and returns its path, so
    /// [`ScriptRuntime::start_script`] (which reads scripts from disk, like the real editor
    /// will) has something real to compile.
    fn write_script(name: &str, source: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("libdqg_scripting_tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, source).unwrap();
        path
    }

    #[test]
    fn on_update_translates_entity_and_persists_state_across_frames() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Mover", Transform::default());
        let script_path = write_script(
            "mover.rhai",
            r#"
            let speed = 1.0;
            let on_update = |dt, input| {
                entity.translate(speed * dt, 0.0, 0.0);
            };
            "#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = ScriptRuntime::new();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let input = no_input();
        assert!(runtime.update_entity(&mut world, entity, 0.5, &input).is_empty());
        assert!(runtime.update_entity(&mut world, entity, 0.5, &input).is_empty());

        let position = world.transforms.get(entity).unwrap().position;
        assert!((position.x - 1.0).abs() < 1e-5, "expected x\u{2248}1.0 after two 0.5s updates, got {position:?}");
    }

    #[test]
    fn repeated_runtime_errors_disable_the_attachment() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Broken", Transform::default());
        let script_path = write_script("broken.rhai", "let on_update = |dt, input| { throw \"boom\"; };");
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = ScriptRuntime::new();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let input = no_input();
        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            let errors = runtime.update_entity(&mut world, entity, 0.1, &input);
            assert_eq!(errors.len(), 1);
        }

        let attachment_enabled = world.scripts.get(entity).unwrap().0[0].enabled;
        assert!(!attachment_enabled, "attachment should auto-disable after MAX_CONSECUTIVE_FAILURES errors");

        // Disabled, so no further errors should be produced even though the script still throws.
        assert!(runtime.update_entity(&mut world, entity, 0.1, &input).is_empty());
    }

    #[test]
    fn on_update_sees_held_and_pressed_keys() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Listener", Transform::default());
        let script_path = write_script(
            "input.rhai",
            r#"
            let on_update = |dt, input| {
                if input.is_held("KeyW") {
                    entity.translate(0.0, 0.0, -1.0);
                }
                if input.is_pressed("Space") {
                    entity.translate(1.0, 0.0, 0.0);
                }
                // An unrecognized key name should just read as not held/pressed, not error.
                if input.is_held("NotAKey") {
                    entity.translate(0.0, 99.0, 0.0);
                }
            };
            "#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = ScriptRuntime::new();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let input = InputState::for_test([KeyCode::KeyW], [KeyCode::Space]);
        assert!(runtime.update_entity(&mut world, entity, 1.0, &input).is_empty());

        let position = world.transforms.get(entity).unwrap().position;
        assert!(
            position.abs_diff_eq(Vec3::new(1.0, 0.0, -1.0), 1e-5),
            "expected held KeyW and pressed Space to both apply, got {position:?}"
        );
    }

    /// Regression test for a gimbal-lock bug: `update_entity` used to sync rotation into the
    /// script's `ScriptApi` by decomposing the entity's `Quat` to Euler angles every frame
    /// (`to_euler`/`from_euler` round-tripped each call), which visibly stalled a continuous
    /// single-axis rotation once it crossed 90° (the middle Euler axis is extracted via `asin`,
    /// capped at \u{b1}90\u{b0}). `rotation` is now carried as a `Quat` end to end (see
    /// `ScriptApi::rotation`'s doc comment), so accumulating well past 90\u{b0} must still produce
    /// the correct final orientation.
    #[test]
    fn on_update_rotation_does_not_stall_past_ninety_degrees() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Spinner", Transform::default());
        let script_path = write_script(
            "spin.rhai",
            r#"
            let speed = 1.0;
            let on_update = |dt, input| {
                entity.rotate(0.0, speed * dt, 0.0);
            };
            "#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = ScriptRuntime::new();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        // Accumulate a full half turn (pi radians) about Y in small per-frame steps.
        let input = no_input();
        let steps = 200;
        let dt = std::f32::consts::PI / steps as f32;
        for _ in 0..steps {
            assert!(runtime.update_entity(&mut world, entity, dt, &input).is_empty());
        }

        let rotation = world.transforms.get(entity).unwrap().rotation;
        let rotated_x = rotation * Vec3::X;
        assert!(
            rotated_x.abs_diff_eq(-Vec3::X, 1e-3),
            "expected a 180\u{b0} Y rotation to flip +X to -X, got {rotated_x:?} (rotation stalled around 90\u{b0}?)"
        );
    }
}
