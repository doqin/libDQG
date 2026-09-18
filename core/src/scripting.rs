use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use glam::{Quat, Vec3};
use rhai::{CustomType, Dynamic, Engine, FnPtr, Scope, TypeBuilder, AST};
use serde::{Deserialize, Serialize};

use crate::ecs::{ComponentStore, Entity};
use crate::input::{InputState, MouseState};
use crate::renderer::{Model, Renderer, Sprite, Texture};
use crate::types::KeyCode;
use crate::world::{Name, Renderable, Transform, World};

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

/// Identifies which entity a [`WorldCommand`] targets: either a real, already-allocated
/// [`Entity`], or one queued for spawn earlier in the same [`ScriptRuntime::drain_commands`]
/// batch and not yet resolved to a real handle (see [`ScriptWorld::spawn`]/
/// [`WorldCommand::Spawn`]).
#[derive(Clone, Copy)]
enum ScriptEntityId {
    Real(Entity),
    Pending(u64),
}

/// An intent queued by an [`EntityHandle`]/[`ScriptWorld`] method, applied by
/// [`ScriptRuntime::drain_commands`] immediately after the `on_start`/`on_update` call that
/// queued it returns. Nothing reachable from inside a Rhai call can hold a live `&mut World` —
/// `rhai`'s custom types must be `Clone + 'static`, which a borrow isn't — so this queue (a
/// cheap `Rc<RefCell<...>>` handle) is how scripts affect `World` anyway: they record an intent
/// here instead of mutating directly, and the host (which *does* have `&mut World` right around
/// each script call) applies it afterward. Applying strictly in emission order means later
/// commands targeting an entity a prior command in the same batch already despawned simply
/// resolve to nothing and no-op, rather than needing special-case handling.
enum WorldCommand {
    SetTransform(ScriptEntityId, Transform),
    Rename(ScriptEntityId, String),
    Despawn(ScriptEntityId),
    Spawn { pending_id: u64, name: String, transform: Transform },
    AttachScript(ScriptEntityId, PathBuf),
    SetScriptEnabled(ScriptEntityId, usize, bool),
    /// Loads a texture at `path` (project-relative) and attaches it as a [`Renderable::Sprite`],
    /// replacing whatever renderable (if any) the entity already had — "attach" and "swap" are
    /// the same command, just depending on whether there was one before. Unlike every other
    /// [`WorldCommand`], applying this needs a live [`Renderer`], so it's only handled when
    /// [`ScriptRuntime::drain_commands`] is given one (see that method's doc comment).
    SetSprite(ScriptEntityId, PathBuf),
    /// The [`Renderable::Model`] counterpart to [`WorldCommand::SetSprite`] — same path
    /// resolution, same renderer requirement.
    SetModel(ScriptEntityId, PathBuf),
    /// Removes whatever renderable the entity has, if any. Unlike `SetSprite`/`SetModel`, this
    /// needs no [`Renderer`] — there's nothing to load.
    ClearRenderable(ScriptEntityId),
    /// Marks/unmarks the entity as persistent across a scene change — see [`World::set_persistent`].
    /// Applied immediately, unlike `ChangeScene`, since it's just a component flag with nothing
    /// else to load.
    SetPersistent(ScriptEntityId, bool),
    /// Requests a scene change: tear down every non-persistent entity, then load and spawn every
    /// `EntityRecord` in the `.ron` file at this project-relative path (resolved against
    /// [`ScriptRuntime::project_root`]) — see [`ScriptRuntime::drain_commands`]'s handling for the
    /// full sequence. If more than one `ChangeScene` lands in the same `drain_commands` batch, the
    /// last one applied wins (each fully tears down and reloads over what the previous one just
    /// loaded) — wasteful but not unsound, and not expected outside pathological scripts.
    ChangeScene(PathBuf),
}

/// A read-only copy of every entity's name and [`Transform`], captured once per frame by
/// [`ScriptRuntime::begin_frame`] before any script runs that frame. `world.find(name)` reads
/// only this, never the live `World` — for the same reason writes go through [`WorldCommand`]
/// instead of a live reference. This means a cross-entity read reflects that entity's state as
/// of the *start* of the frame, even if another script already moved it earlier the same frame —
/// deterministic and simple to reason about, at the cost of at-most-one-frame staleness for
/// reads of entities other than self. Self (`entity`) isn't affected by this: it's resynced from
/// the live `World` before every call, same as always (see [`ScriptRuntime::update_entity`]).
#[derive(Default)]
struct FrameSnapshot {
    transforms: ComponentStore<Transform>,
    names: ComponentStore<Name>,
    /// First entity (by spawn order) wins if two entities share a name.
    by_name: HashMap<String, Entity>,
}

impl FrameSnapshot {
    fn capture(world: &World) -> Self {
        let mut by_name = HashMap::new();
        for (entity, name) in world.names.iter() {
            by_name.entry(name.0.clone()).or_insert(entity);
        }
        Self { transforms: world.transforms.clone(), names: world.names.clone(), by_name }
    }
}

/// A handle to one entity, seen from inside Rhai — either a script's own `entity`, or another
/// entity reached via `world.find(name)`. Both are this same type with the same methods, so
/// there's exactly one entity-manipulation surface rather than two parallel ones.
///
/// Every mutating method (`translate`/`rotate`/`scale`/the `x`/`y`/`z` setters/`set_name`/
/// `despawn`/`attach_script`/`set_script_enabled`) does two things: updates this handle's own
/// cached fields (so a script reading `entity.x` right after setting it still sees its own
/// write, matching how it felt before cross-entity access existed) and pushes a matching
/// [`WorldCommand`] onto `commands` — see that type's doc comment for why nothing here can just
/// mutate `World` directly.
///
/// Registered on the [`Engine`] via `#[derive(CustomType)]` (`ScriptRuntime::build_engine`'s
/// `engine.build_type::<EntityHandle>()`) instead of a hand-written builder chain:
/// - `#[rhai_type(skip)]` opts every field out of the derive's automatic get/set-property
///   registration — `x`/`y`/`z` need custom setters now (to also queue a command), so unlike the
///   original single-entity-only version of this type, nothing here can just be a plain
///   auto-exposed field anymore.
/// - `register_extra` (wired up via `#[rhai_type(extra = Self::register_extra)]` below) is where
///   all of that — properties and methods alike — gets registered, in one place.
#[derive(Clone, CustomType)]
#[rhai_type(name = "Entity", extra = Self::register_extra)]
struct EntityHandle {
    #[rhai_type(skip)]
    x: f64,
    #[rhai_type(skip)]
    y: f64,
    #[rhai_type(skip)]
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
    #[rhai_type(skip)]
    id: ScriptEntityId,
    #[rhai_type(skip)]
    commands: Rc<RefCell<Vec<WorldCommand>>>,
}

impl EntityHandle {
    /// Everything the field-level `#[rhai_type]` attributes on [`EntityHandle`] can't express as
    /// a plain property: the `x`/`y`/`z` get/set pair (custom, not auto, since the setters must
    /// also queue a [`WorldCommand::SetTransform`]) and the `translate`/`rotate`/`look_at`/
    /// `scale`/`name`/`set_name`/`despawn`/`set_persistent`/`attach_script`/`set_script_enabled`/
    /// `set_sprite`/`set_model`/`detach_renderable` methods.
    fn register_extra(builder: &mut TypeBuilder<Self>) {
        builder
            .with_get_set("x", |e: &mut Self| e.x, |e: &mut Self, v: f64| {
                e.x = v;
                e.queue_transform_update();
            })
            .with_get_set("y", |e: &mut Self| e.y, |e: &mut Self, v: f64| {
                e.y = v;
                e.queue_transform_update();
            })
            .with_get_set("z", |e: &mut Self| e.z, |e: &mut Self, v: f64| {
                e.z = v;
                e.queue_transform_update();
            })
            .with_fn("translate", |e: &mut Self, x: f64, y: f64, z: f64| {
                e.x += x;
                e.y += y;
                e.z += z;
                e.queue_transform_update();
            })
            .with_fn("rotate", |e: &mut Self, x: f64, y: f64, z: f64| {
                let delta = Quat::from_euler(glam::EulerRot::XYZ, x as f32, y as f32, z as f32);
                e.rotation = (e.rotation * delta).normalize();
                e.queue_transform_update();
            })
            .with_fn("look_at", |e: &mut Self, x: f64, y: f64, z: f64| {
                let position = Vec3::new(e.x as f32, e.y as f32, e.z as f32);
                let direction = Vec3::new(x as f32, y as f32, z as f32) - position;
                // A zero-length direction (looking at your own position) has no meaningful
                // rotation to face — leave the current rotation alone rather than feeding
                // `from_rotation_arc` a NaN from normalizing a zero vector.
                if direction.length_squared() > 1e-12 {
                    // Sets rotation absolutely (unlike `rotate`, which composes a delta) so the
                    // entity faces `(x, y, z)` down its local -Z axis — the same "forward" every
                    // camera entity's `CameraComponent` assumes (see `world::CameraComponent`),
                    // so this doubles as "point this camera at" for a camera entity.
                    e.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction.normalize());
                    e.queue_transform_update();
                }
            })
            .with_fn("scale", |e: &mut Self, x: f64, y: f64, z: f64| {
                e.scale *= Vec3::new(x as f32, y as f32, z as f32);
                e.queue_transform_update();
            })
            .with_fn("name", |e: &mut Self| e.name.clone())
            .with_fn("set_name", |e: &mut Self, name: &str| {
                e.name = name.to_string();
                e.commands.borrow_mut().push(WorldCommand::Rename(e.id, e.name.clone()));
            })
            .with_fn("despawn", |e: &mut Self| {
                e.commands.borrow_mut().push(WorldCommand::Despawn(e.id));
            })
            .with_fn("set_persistent", |e: &mut Self, persistent: bool| {
                e.commands.borrow_mut().push(WorldCommand::SetPersistent(e.id, persistent));
            })
            .with_fn("attach_script", |e: &mut Self, path: &str| {
                e.commands.borrow_mut().push(WorldCommand::AttachScript(e.id, PathBuf::from(path)));
            })
            .with_fn("set_script_enabled", |e: &mut Self, index: i64, enabled: bool| {
                e.commands.borrow_mut().push(WorldCommand::SetScriptEnabled(e.id, index.max(0) as usize, enabled));
            })
            .with_fn("set_sprite", |e: &mut Self, path: &str| {
                e.commands.borrow_mut().push(WorldCommand::SetSprite(e.id, PathBuf::from(path)));
            })
            .with_fn("set_model", |e: &mut Self, path: &str| {
                e.commands.borrow_mut().push(WorldCommand::SetModel(e.id, PathBuf::from(path)));
            })
            .with_fn("detach_renderable", |e: &mut Self| {
                e.commands.borrow_mut().push(WorldCommand::ClearRenderable(e.id));
            });
    }

    /// Builds a handle for `id`, seeded from `transform`/`name` — used both for a script's own
    /// `entity` (seeded from the live `World`) and for a handle `world.find(...)` hands back
    /// (seeded from the frozen [`FrameSnapshot`]).
    fn new(id: ScriptEntityId, transform: &Transform, name: String, commands: Rc<RefCell<Vec<WorldCommand>>>) -> Self {
        Self {
            x: transform.position.x as f64,
            y: transform.position.y as f64,
            z: transform.position.z as f64,
            rotation: transform.rotation,
            scale: transform.scale,
            name,
            id,
            commands,
        }
    }

    /// Overwrites position/rotation/scale/name from the live `World` — the one place
    /// [`ScriptRuntime::update_entity`] needs to sync a script's own entity in before calling
    /// `on_update`, rather than four separate per-field copies. Only ever used for self: a
    /// handle from `world.find(...)` is seeded once from the frame's frozen snapshot and never
    /// resynced (see the [`FrameSnapshot`] doc comment for why that's deliberate).
    fn sync_from_world(&mut self, transform: &Transform, name: &str) {
        self.x = transform.position.x as f64;
        self.y = transform.position.y as f64;
        self.z = transform.position.z as f64;
        self.rotation = transform.rotation;
        self.scale = transform.scale;
        self.name = name.to_string();
    }

    fn queue_transform_update(&self) {
        let transform = Transform {
            position: Vec3::new(self.x as f32, self.y as f32, self.z as f32),
            rotation: self.rotation,
            scale: self.scale,
        };
        self.commands.borrow_mut().push(WorldCommand::SetTransform(self.id, transform));
    }
}

/// The `world` object scripts use to reach entities other than their own:
/// `world.find(name)` (an [`EntityHandle`], or `()` if no entity has that name — Rhai has no
/// `Option` visible to scripts, so unit is the idiomatic "not found", the same way
/// [`ScriptInput`]'s `is_held`/`is_pressed` treat an unrecognized key name as simply false rather
/// than an error) and `world.spawn_entity(name, x, y, z)` (`spawn` alone is a reserved word in
/// Rhai, even as a method name — queues a new entity, see
/// [`WorldCommand::Spawn`] — and returns a handle to it usable immediately, since
/// [`ScriptRuntime::drain_commands`] resolves same-batch pending spawns before anything later in
/// the batch that references them).
///
/// Reads (`find`) come from the frozen [`FrameSnapshot`]; writes (`spawn`, and anything called on
/// a handle it returns) go through the same [`WorldCommand`] queue as `entity` does — see both of
/// those types' doc comments for why.
#[derive(Clone, CustomType)]
#[rhai_type(name = "World", extra = Self::register_extra)]
struct ScriptWorld {
    #[rhai_type(skip)]
    frame: Rc<RefCell<FrameSnapshot>>,
    #[rhai_type(skip)]
    commands: Rc<RefCell<Vec<WorldCommand>>>,
    #[rhai_type(skip)]
    next_pending_id: Rc<Cell<u64>>,
}

impl ScriptWorld {
    fn register_extra(builder: &mut TypeBuilder<Self>) {
        builder
            .with_fn("find", |w: &mut Self, name: &str| -> Dynamic {
                let frame = w.frame.borrow();
                let Some(&entity) = frame.by_name.get(name) else { return Dynamic::UNIT };
                let transform = frame.transforms.get(entity).copied().unwrap_or_default();
                let entity_name = frame.names.get(entity).map(|n| n.0.clone()).unwrap_or_default();
                Dynamic::from(EntityHandle::new(ScriptEntityId::Real(entity), &transform, entity_name, w.commands.clone()))
            })
            .with_fn("spawn_entity", |w: &mut Self, name: &str, x: f64, y: f64, z: f64| -> EntityHandle {
                let pending_id = w.next_pending_id.get();
                w.next_pending_id.set(pending_id + 1);
                let transform = Transform { position: Vec3::new(x as f32, y as f32, z as f32), ..Transform::default() };
                w.commands.borrow_mut().push(WorldCommand::Spawn { pending_id, name: name.to_string(), transform });
                EntityHandle::new(ScriptEntityId::Pending(pending_id), &transform, name.to_string(), w.commands.clone())
            });
    }
}

/// The `scene` object scripts use to change the loaded scene:
/// `scene.change("scenes/level2.ron")` (project-relative path to a `.ron` [`crate::scene_file::SceneFile`]).
/// A thin wrapper over the shared [`WorldCommand`] queue — see that type's `ChangeScene` variant
/// and [`EntityHandle`]'s doc comment for why nothing here can mutate `World` directly.
#[derive(Clone, CustomType)]
#[rhai_type(name = "Scene", extra = Self::register_extra)]
struct ScriptScene {
    #[rhai_type(skip)]
    commands: Rc<RefCell<Vec<WorldCommand>>>,
}

impl ScriptScene {
    fn register_extra(builder: &mut TypeBuilder<Self>) {
        builder.with_fn("change", |s: &mut Self, path: &str| {
            s.commands.borrow_mut().push(WorldCommand::ChangeScene(PathBuf::from(path)));
        });
    }
}

/// A read-only keyboard/mouse snapshot passed as `on_update`'s second argument:
/// `let on_update = |dt, input| { if input.is_held("KeyW") { entity.translate(0.0, 0.0, -dt); } };`.
/// Key names match [`KeyCode`]'s own variant identifiers via [`KeyCode::from_name`] (its derived
/// `Debug` output prints the same strings) — e.g. `"KeyW"`, `"ArrowUp"`, `"Space"`,
/// `"ShiftLeft"`. Mouse button names match [`winit::event::MouseButton`]'s own variant
/// identifiers via [`mouse_button_from_name`] — `"Left"`, `"Right"`, `"Middle"`, `"Back"`,
/// `"Forward"`. An unrecognized name is simply never held/pressed rather than an error, so a
/// typo in a key/button name fails quietly instead of aborting the script.
///
/// Rebuilt fresh from [`InputState`]/[`MouseState`] every [`ScriptRuntime::update_entity`] call
/// rather than synced in/out like [`EntityHandle`] — input is read-only from a script's
/// perspective, so there's nothing to write back.
#[derive(Clone, CustomType)]
#[rhai_type(name = "Input", extra = Self::register_extra)]
struct ScriptInput {
    #[rhai_type(skip)]
    held: HashSet<KeyCode>,
    #[rhai_type(skip)]
    pressed: HashSet<KeyCode>,
    #[rhai_type(skip)]
    mouse_held: HashSet<winit::event::MouseButton>,
    #[rhai_type(skip)]
    mouse_pressed: HashSet<winit::event::MouseButton>,
    #[rhai_type(skip)]
    mouse_x: f64,
    #[rhai_type(skip)]
    mouse_y: f64,
    /// Mouse movement since last frame — the usual building block for a mouse-look camera.
    #[rhai_type(skip)]
    mouse_dx: f64,
    #[rhai_type(skip)]
    mouse_dy: f64,
    #[rhai_type(skip)]
    mouse_wheel: f64,
}

impl ScriptInput {
    fn from_state(input: &InputState, mouse: &MouseState, mouse_delta: (f32, f32)) -> Self {
        let (mouse_x, mouse_y) = mouse.position();
        Self {
            held: input.keys_held().collect(),
            pressed: input.keys_pressed().collect(),
            mouse_held: mouse.buttons_held().collect(),
            mouse_pressed: mouse.buttons_pressed().collect(),
            mouse_x: mouse_x as f64,
            mouse_y: mouse_y as f64,
            mouse_dx: mouse_delta.0 as f64,
            mouse_dy: mouse_delta.1 as f64,
            mouse_wheel: mouse.wheel_delta() as f64,
        }
    }

    fn register_extra(builder: &mut TypeBuilder<Self>) {
        builder
            .with_fn("is_held", |i: &mut Self, name: &str| {
                KeyCode::from_name(name).is_some_and(|key| i.held.contains(&key))
            })
            .with_fn("is_pressed", |i: &mut Self, name: &str| {
                KeyCode::from_name(name).is_some_and(|key| i.pressed.contains(&key))
            })
            .with_fn("is_mouse_held", |i: &mut Self, name: &str| {
                mouse_button_from_name(name).is_some_and(|button| i.mouse_held.contains(&button))
            })
            .with_fn("is_mouse_pressed", |i: &mut Self, name: &str| {
                mouse_button_from_name(name).is_some_and(|button| i.mouse_pressed.contains(&button))
            })
            .with_fn("mouse_x", |i: &mut Self| i.mouse_x)
            .with_fn("mouse_y", |i: &mut Self| i.mouse_y)
            .with_fn("mouse_dx", |i: &mut Self| i.mouse_dx)
            .with_fn("mouse_dy", |i: &mut Self| i.mouse_dy)
            .with_fn("mouse_wheel", |i: &mut Self| i.mouse_wheel);
    }
}

/// Parses a mouse button name matching [`winit::event::MouseButton`]'s own variant identifiers
/// (the same strings its derived `Debug` output prints for its unit variants) — used by
/// [`ScriptInput`] so a `.rhai` script can reference a mouse button by name, the same way
/// [`KeyCode::from_name`] does for the keyboard. `Other(_)` (extra vendor-specific buttons) has
/// no name-based way to reach it, since there's no stable name for an arbitrary button index.
fn mouse_button_from_name(name: &str) -> Option<winit::event::MouseButton> {
    Some(match name {
        "Left" => winit::event::MouseButton::Left,
        "Right" => winit::event::MouseButton::Right,
        "Middle" => winit::event::MouseButton::Middle,
        "Back" => winit::event::MouseButton::Back,
        "Forward" => winit::event::MouseButton::Forward,
        _ => return None,
    })
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
    /// Shared with every script instance's `entity`/`world` values — see [`WorldCommand`].
    commands: Rc<RefCell<Vec<WorldCommand>>>,
    /// Shared with every script instance's `world` value — see [`FrameSnapshot`].
    frame: Rc<RefCell<FrameSnapshot>>,
    /// Shared with every script instance's `world` value, so two scripts spawning in the same
    /// frame can't mint colliding [`ScriptEntityId::Pending`] ids.
    next_pending_id: Rc<Cell<u64>>,
    /// Needed to resolve `attach_script`'s project-relative path into a real file path — see
    /// [`WorldCommand::AttachScript`]. `start_script`'s own `script_path` argument, by contrast,
    /// always arrives already resolved (the editor does that itself before calling it).
    project_root: PathBuf,
}

impl ScriptRuntime {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            engine: Self::build_engine(),
            compiled: HashMap::new(),
            instances: HashMap::new(),
            commands: Rc::new(RefCell::new(Vec::new())),
            frame: Rc::new(RefCell::new(FrameSnapshot::default())),
            next_pending_id: Rc::new(Cell::new(0)),
            project_root,
        }
    }

    fn build_engine() -> Engine {
        let mut engine = Engine::new();
        engine.build_type::<EntityHandle>();
        engine.build_type::<ScriptInput>();
        engine.build_type::<ScriptWorld>();
        engine.build_type::<ScriptScene>();
        engine
    }

    /// Refreshes the [`FrameSnapshot`] every `world.find(...)` call this frame will read from.
    /// Call once per frame, before running any script that frame (see the editor's
    /// `EditorScene::update`, `EditorMode::Playing` branch).
    pub fn begin_frame(&mut self, world: &World) {
        *self.frame.borrow_mut() = FrameSnapshot::capture(world);
    }

    fn script_world(&self) -> ScriptWorld {
        ScriptWorld { frame: self.frame.clone(), commands: self.commands.clone(), next_pending_id: self.next_pending_id.clone() }
    }

    fn script_scene(&self) -> ScriptScene {
        ScriptScene { commands: self.commands.clone() }
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
    ///
    /// Anything `on_start` queues via `entity`/`world` (a despawn, a spawn, ...) is *not* applied
    /// here — it sits in the shared command queue until the next [`Self::drain_commands`] call
    /// (the first `update_entity` of the next frame), one frame later than an `on_update`-queued
    /// command would be. This only matters for `on_start`-time cross-entity effects, which are
    /// rare enough not to be worth a `&mut World` here just to close that one-frame gap.
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
        scope.push("entity", EntityHandle::new(ScriptEntityId::Real(entity), &transform, name, self.commands.clone()));
        scope.push("world", self.script_world());
        scope.push("scene", self.script_scene());

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

    /// Starts every enabled script attachment (in attachment order) that isn't already running on
    /// an entity currently in `world` — the "start scripts" half of loading a scene, factored out
    /// so `EditorScene::start_play`, `runtime`'s `RuntimeScene::start`, and a script-triggered
    /// `scene.change(...)` (see [`WorldCommand::ChangeScene`]) all run it identically instead of
    /// each keeping its own copy of this loop. The "isn't already running" check is what lets a
    /// scene change call this over the *whole* post-change `world` (persistent entities included)
    /// without restarting a persistent entity's already-running script and losing its state.
    /// `base_dir` resolves each attachment's project-relative `path` into a real file path (an
    /// editor project's root, or an exported game's `res/` directory). Caller must call
    /// [`Self::begin_frame`] first so `world.find(...)` inside any `on_start` sees a snapshot that
    /// includes the entities being started (see
    /// `runtime_tests::on_start_can_read_other_entities_via_find_when_begin_frame_ran_first`).
    pub fn start_all_scripts(&mut self, world: &World, base_dir: &Path) -> Vec<ScriptError> {
        let mut errors = Vec::new();
        for entity in world.iter_entities().collect::<Vec<_>>() {
            let Some(list) = world.scripts.get(entity) else { continue };
            // Skip an attachment that already has a running `ScriptInstance` — load-bearing for
            // `WorldCommand::ChangeScene`, whose surviving persistent entities are still in
            // `world` (and still have their `ScriptList`) but must keep their existing instance
            // (and its accumulated state) rather than being restarted from scratch alongside the
            // scene's actually-new entities.
            let starts: Vec<(usize, PathBuf)> = list
                .0
                .iter()
                .enumerate()
                .filter(|(index, attachment)| attachment.enabled && !self.instances.contains_key(&(entity, *index)))
                .map(|(index, attachment)| (index, base_dir.join(&attachment.path)))
                .collect();

            for (index, script_path) in starts {
                if let Err(e) = self.start_script(world, entity, index, &script_path) {
                    errors.push(e);
                }
            }
        }
        errors
    }

    /// Calls every started, enabled attachment's `on_update(dt, input)` closure for `entity`, in
    /// attachment order (disabled attachments — including ones this call itself just disabled
    /// after too many failures — are skipped), syncing the entity's [`crate::world::Transform`]
    /// and name in beforehand and applying whatever it queued via `entity`/`world` (see
    /// [`Self::drain_commands`]) right after. One instance erroring doesn't stop the others;
    /// every error encountered is returned rather than the first one short-circuiting. If an
    /// attachment despawns its own entity, the remaining attachments on that entity are skipped
    /// for the rest of this call — there's nothing left to update.
    ///
    /// `renderer` is only needed for `entity.set_sprite(...)`/`set_model(...)` (loading a new
    /// asset needs a live GPU device); pass `None` when one isn't available (e.g. in a test with
    /// no real `Renderer`) and any such command just reports a [`ScriptError`] instead of
    /// silently doing nothing or panicking.
    ///
    /// `mouse_delta` is the caller's responsibility, same as it is for
    /// `editor::fly_camera::FlyCamera::update` — `MouseState` only tracks position, not
    /// frame-to-frame movement, so the caller (which already diffs `MouseState::position()`
    /// against last frame's for its own fly camera) passes it through directly.
    pub fn update_entity(
        &mut self,
        world: &mut World,
        entity: Entity,
        dt: f32,
        input: &InputState,
        mouse: &MouseState,
        mouse_delta: (f32, f32),
        renderer: Option<&Renderer>,
    ) -> Vec<ScriptError> {
        let mut errors = Vec::new();
        let Some(attachments) = world.scripts.get(entity).cloned() else { return errors };
        let input = ScriptInput::from_state(input, mouse, mouse_delta);

        for (index, attachment) in attachments.0.iter().enumerate() {
            if !attachment.enabled {
                continue;
            }
            let Some(instance) = self.instances.get_mut(&(entity, index)) else { continue };
            let Some(on_update) = instance.on_update.clone() else { continue };

            // Sync the entity's transform/name into the script's own `entity` handle in one shot.
            if let Some(transform) = world.transforms.get(entity).copied() {
                let name = world.names.get(entity).map(|n| n.0.as_str()).unwrap_or("");
                if let Some(mut handle) = scope_entity_mut(&mut instance.scope) {
                    handle.sync_from_world(&transform, name);
                }
            }

            // Call the script's `on_update(dt, input)` closure, which may mutate its captured
            // `Scope` variables and/or queue `WorldCommand`s via `entity`/`world`.
            match on_update.call::<rhai::Dynamic>(&self.engine, &instance.ast, (dt as f64, input.clone())) {
                Ok(_) => instance.consecutive_failures = 0,
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

            // Apply whatever this call queued — commands emitted before a thrown error are still
            // valid intents and still apply, since Rhai only halts the *script*, not the queue.
            errors.extend(self.drain_commands(world, entity, renderer, &attachment.path));

            if !world.is_alive(entity) {
                break;
            }
        }

        errors
    }

    /// Applies every [`WorldCommand`] queued since the last drain, in emission order, resolving
    /// any [`ScriptEntityId::Pending`] against `Spawn` commands earlier in this same batch. This
    /// is the only place `World` is actually mutated on a script's behalf — nothing reachable
    /// from inside the Rhai call itself can hold `world` (see [`WorldCommand`]'s doc comment).
    /// `renderer` is threaded through for `SetSprite`/`SetModel` (see [`Self::update_entity`]'s
    /// doc comment); `source_script` attributes any error that isn't more specifically
    /// attributable (e.g. an `AttachScript` failure already carries its own script path).
    /// `entity` attributes a `ChangeScene` error, which has no `ScriptEntityId` of its own (it's
    /// issued via the entity-less `scene` global, not a handle to a specific entity).
    fn drain_commands(&mut self, world: &mut World, entity: Entity, renderer: Option<&Renderer>, source_script: &Path) -> Vec<ScriptError> {
        let commands: Vec<WorldCommand> = self.commands.borrow_mut().drain(..).collect();
        let mut pending: HashMap<u64, Entity> = HashMap::new();
        let mut errors = Vec::new();

        let resolve = |id: ScriptEntityId, pending: &HashMap<u64, Entity>| match id {
            ScriptEntityId::Real(entity) => Some(entity),
            ScriptEntityId::Pending(pending_id) => pending.get(&pending_id).copied(),
        };

        for command in commands {
            match command {
                WorldCommand::Spawn { pending_id, name, transform } => {
                    let entity = world.spawn_empty(name, transform);
                    pending.insert(pending_id, entity);
                }
                WorldCommand::SetTransform(id, transform) => {
                    if let Some(entity) = resolve(id, &pending) {
                        if let Some(slot) = world.transforms.get_mut(entity) {
                            *slot = transform;
                        }
                    }
                }
                WorldCommand::Rename(id, name) => {
                    if let Some(entity) = resolve(id, &pending) {
                        if let Some(slot) = world.names.get_mut(entity) {
                            slot.0 = name;
                        }
                    }
                }
                WorldCommand::Despawn(id) => {
                    if let Some(entity) = resolve(id, &pending) {
                        world.despawn(entity);
                        self.instances.retain(|(e, _), _| *e != entity);
                    }
                }
                WorldCommand::AttachScript(id, path) => {
                    if let Some(entity) = resolve(id, &pending) {
                        let full_path = self.project_root.join(&path);
                        let new_index = match world.scripts.get_mut(entity) {
                            Some(list) => {
                                list.0.push(ScriptAttachment { path, enabled: true });
                                list.0.len() - 1
                            }
                            None => {
                                world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path, enabled: true }]));
                                0
                            }
                        };
                        // Start it right away so it actually runs this same Play session,
                        // instead of sitting inert until the next Stop/Play like an attachment
                        // made through the Inspector UI does today.
                        if let Err(e) = self.start_script(world, entity, new_index, &full_path) {
                            errors.push(e);
                        }
                    }
                }
                WorldCommand::SetScriptEnabled(id, index, enabled) => {
                    if let Some(entity) = resolve(id, &pending) {
                        if let Some(list) = world.scripts.get_mut(entity) {
                            if let Some(attachment) = list.0.get_mut(index) {
                                attachment.enabled = enabled;
                            }
                        }
                    }
                }
                WorldCommand::SetSprite(id, path) => {
                    if let Some(entity) = resolve(id, &pending) {
                        let Some(renderer) = renderer else {
                            errors.push(no_renderer_error(entity, source_script, &path));
                            continue;
                        };
                        let full_path = self.project_root.join(&path);
                        match Texture::from_path(renderer, &full_path) {
                            Ok(texture) => {
                                let mut sprite = Sprite::new(Arc::new(texture));
                                sprite.fit_within_unit_square();
                                world.set_renderable(entity, Renderable::Sprite(sprite));
                            }
                            Err(e) => errors.push(ScriptError {
                                entity,
                                script: source_script.to_path_buf(),
                                message: format!("failed to load sprite {}: {e}", path.display()),
                            }),
                        }
                    }
                }
                WorldCommand::SetModel(id, path) => {
                    if let Some(entity) = resolve(id, &pending) {
                        let Some(renderer) = renderer else {
                            errors.push(no_renderer_error(entity, source_script, &path));
                            continue;
                        };
                        let full_path = self.project_root.join(&path);
                        match Model::load(renderer, &full_path) {
                            Ok(model) => world.set_renderable(entity, Renderable::Model(model)),
                            Err(e) => errors.push(ScriptError {
                                entity,
                                script: source_script.to_path_buf(),
                                message: format!("failed to load model {}: {e}", path.display()),
                            }),
                        }
                    }
                }
                WorldCommand::ClearRenderable(id) => {
                    if let Some(entity) = resolve(id, &pending) {
                        world.clear_renderable(entity);
                    }
                }
                WorldCommand::SetPersistent(id, persistent) => {
                    if let Some(target) = resolve(id, &pending) {
                        world.set_persistent(target, persistent);
                    }
                }
                WorldCommand::ChangeScene(path) => {
                    let full_path = self.project_root.join(&path);
                    match crate::scene_file::SceneFile::load(&full_path) {
                        Ok(scene_file) => {
                            world.despawn_non_persistent();
                            self.instances.retain(|(e, _), _| world.is_alive(*e));

                            // Clone project_root into a local first: start_all_scripts/begin_frame
                            // both need `&mut self` right below, so `&self.project_root` can't be
                            // borrowed alongside them.
                            let base_dir = self.project_root.clone();
                            for record in scene_file.entities {
                                record.spawn_into(world, renderer, &base_dir);
                            }
                            self.begin_frame(world);
                            errors.extend(self.start_all_scripts(world, &base_dir));
                        }
                        Err(e) => errors.push(ScriptError {
                            entity,
                            script: source_script.to_path_buf(),
                            message: format!("failed to load scene {}: {e}", path.display()),
                        }),
                    }
                }
            }
        }

        errors
    }
}

fn no_renderer_error(entity: Entity, source_script: &Path, asset_path: &Path) -> ScriptError {
    ScriptError {
        entity,
        script: source_script.to_path_buf(),
        message: format!("cannot load {} — no renderer available", asset_path.display()),
    }
}

/// Borrows the `entity` variable out of a script instance's `Scope` as `&mut EntityHandle`,
/// working whether or not it's been promoted to a shared value by closure capture (see
/// `scripting::closure_state_spike`).
fn scope_entity_mut<'a>(scope: &'a mut Scope<'static>) -> Option<rhai::DynamicWriteLock<'a, EntityHandle>> {
    scope.get_mut("entity")?.write_lock::<EntityHandle>()
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
        Camera { position: Vec3::ZERO, yaw: 0.0, pitch: 0.0, aspect: 1.0, fov: 45.0, znear: 0.1, zfar: 100.0 }
    }

    /// An [`InputState`] with nothing held/pressed, for tests that don't care about input.
    fn no_input() -> InputState {
        InputState::for_test([], [])
    }

    /// A [`MouseState`] with nothing held/pressed/moved, for tests that don't care about mouse
    /// input.
    fn no_mouse() -> MouseState {
        MouseState::new()
    }

    fn test_runtime() -> ScriptRuntime {
        ScriptRuntime::new(PathBuf::new())
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

    /// Serializes `scene_file` to a fresh temp `.ron` file and returns its path, so
    /// `WorldCommand::ChangeScene`'s handling (which reads a scene file from disk, like the real
    /// editor/runtime will) has something real to load.
    fn write_scene(name: &str, scene_file: &crate::scene_file::SceneFile) -> PathBuf {
        let dir = std::env::temp_dir().join("libdqg_scripting_tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        scene_file.save(&path).unwrap();
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

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let input = no_input();
        assert!(runtime.update_entity(&mut world, entity, 0.5, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        assert!(runtime.update_entity(&mut world, entity, 0.5, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        let position = world.transforms.get(entity).unwrap().position;
        assert!((position.x - 1.0).abs() < 1e-5, "expected x\u{2248}1.0 after two 0.5s updates, got {position:?}");
    }

    #[test]
    fn repeated_runtime_errors_disable_the_attachment() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Broken", Transform::default());
        let script_path = write_script("broken.rhai", "let on_update = |dt, input| { throw \"boom\"; };");
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let input = no_input();
        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            let errors = runtime.update_entity(&mut world, entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None);
            assert_eq!(errors.len(), 1);
        }

        let attachment_enabled = world.scripts.get(entity).unwrap().0[0].enabled;
        assert!(!attachment_enabled, "attachment should auto-disable after MAX_CONSECUTIVE_FAILURES errors");

        // Disabled, so no further errors should be produced even though the script still throws.
        assert!(runtime.update_entity(&mut world, entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
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

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let input = InputState::for_test([KeyCode::KeyW], [KeyCode::Space]);
        assert!(runtime.update_entity(&mut world, entity, 1.0, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        let position = world.transforms.get(entity).unwrap().position;
        assert!(
            position.abs_diff_eq(Vec3::new(1.0, 0.0, -1.0), 1e-5),
            "expected held KeyW and pressed Space to both apply, got {position:?}"
        );
    }

    #[test]
    fn on_update_sees_mouse_state() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("MouseListener", Transform::default());
        let script_path = write_script(
            "mouse.rhai",
            r#"
            let on_update = |dt, input| {
                if input.is_mouse_held("Left") {
                    entity.translate(0.0, 0.0, -1.0);
                }
                if input.is_mouse_pressed("Right") {
                    entity.translate(1.0, 0.0, 0.0);
                }
                entity.translate(input.mouse_dx() * 0.1, input.mouse_dy() * 0.1, 0.0);
                // An unrecognized button name should just read as not held/pressed, not error.
                if input.is_mouse_held("NotAButton") {
                    entity.translate(0.0, 99.0, 0.0);
                }
            };
            "#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        let mouse = MouseState::for_test(
            (0.0, 0.0),
            [winit::event::MouseButton::Left],
            [winit::event::MouseButton::Right],
            0.0,
        );
        assert!(runtime.update_entity(&mut world, entity, 1.0, &no_input(), &mouse, (5.0, 2.0), None).is_empty());

        let position = world.transforms.get(entity).unwrap().position;
        assert!(
            position.abs_diff_eq(Vec3::new(1.5, 0.2, -1.0), 1e-5),
            "expected held Left, pressed Right, and mouse delta to all apply, got {position:?}"
        );
    }

    /// Regression test for a gimbal-lock bug: `update_entity` used to sync rotation into the
    /// script's entity handle by decomposing the entity's `Quat` to Euler angles every frame
    /// (`to_euler`/`from_euler` round-tripped each call), which visibly stalled a continuous
    /// single-axis rotation once it crossed 90° (the middle Euler axis is extracted via `asin`,
    /// capped at ±90°). `rotation` is now carried as a `Quat` end to end (see
    /// [`EntityHandle::rotation`]'s doc comment), so accumulating well past 90° must still
    /// produce the correct final orientation.
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

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");

        // Accumulate a full half turn (pi radians) about Y in small per-frame steps.
        let input = no_input();
        let steps = 200;
        let dt = std::f32::consts::PI / steps as f32;
        for _ in 0..steps {
            assert!(runtime.update_entity(&mut world, entity, dt, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        }

        let rotation = world.transforms.get(entity).unwrap().rotation;
        let rotated_x = rotation * Vec3::X;
        assert!(
            rotated_x.abs_diff_eq(-Vec3::X, 1e-3),
            "expected a 180\u{b0} Y rotation to flip +X to -X, got {rotated_x:?} (rotation stalled around 90\u{b0}?)"
        );
    }

    #[test]
    fn look_at_faces_the_target_along_local_negative_z() {
        let mut world = World::new(test_camera());
        let looker = world.spawn_empty("Looker", Transform { position: Vec3::new(5.0, 0.0, 0.0), ..Transform::default() });
        let script_path = write_script(
            "look_at.rhai",
            r#"
            let on_update = |dt, input| {
                entity.look_at(0.0, 0.0, 0.0);
            };
            "#,
        );
        world.scripts.insert(looker, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, looker, 0, &script_path).expect("script should start");
        assert!(runtime.update_entity(&mut world, looker, 0.1, &no_input(), &no_mouse(), (0.0, 0.0), None).is_empty());

        let rotation = world.transforms.get(looker).unwrap().rotation;
        let forward = rotation * Vec3::NEG_Z;
        assert!(
            forward.abs_diff_eq(Vec3::NEG_X, 1e-4),
            "expected the entity at (5,0,0) to face back toward the origin along -X, got forward={forward:?}"
        );
    }

    /// Regression test: `world.find(...)` reads from the [`FrameSnapshot`] `begin_frame` last
    /// captured, but `on_start` used to be reachable (via `start_script`) *before* any
    /// `begin_frame` call ever ran, so `world.find` inside an `on_start` hook always saw the
    /// default-empty snapshot and returned `()` no matter what. The fix is at the call site
    /// (`editor::EditorScene::start_play` now calls `begin_frame` before its `start_script`
    /// loop), but the contract belongs to `ScriptRuntime` — this locks in that calling
    /// `begin_frame` before `start_script` is sufficient for `on_start` to see other entities.
    #[test]
    fn on_start_can_read_other_entities_via_find_when_begin_frame_ran_first() {
        let mut world = World::new(test_camera());
        world.spawn_empty("Target", Transform { position: Vec3::new(3.0, 0.0, 0.0), ..Transform::default() });
        let reader = world.spawn_empty("Reader", Transform::default());
        let script_path = write_script(
            "read_other_on_start.rhai",
            r#"
            let on_start = || {
                // If `world.find` saw the default-empty snapshot (the bug this test guards
                // against), `other` would be `()` and `.x` would throw here, failing this
                // `on_start` call.
                let other = world.find("Target");
                let x = other.x;
            };
            "#,
        );
        world.scripts.insert(reader, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.begin_frame(&world);

        assert!(runtime.start_script(&world, reader, 0, &script_path).is_ok());
    }

    #[test]
    fn on_update_reads_another_entitys_position_via_find() {
        let mut world = World::new(test_camera());
        world.spawn_empty("Target", Transform { position: Vec3::new(3.0, 0.0, 0.0), ..Transform::default() });
        let reader = world.spawn_empty("Reader", Transform::default());
        let script_path = write_script(
            "read_other.rhai",
            r#"
            let on_update = |dt, input| {
                let other = world.find("Target");
                entity.x = other.x;
            };
            "#,
        );
        world.scripts.insert(reader, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, reader, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, reader, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert_eq!(world.transforms.get(reader).unwrap().position.x, 3.0);
    }

    #[test]
    fn on_update_writes_another_entitys_position_via_find() {
        let mut world = World::new(test_camera());
        let target = world.spawn_empty("Target", Transform::default());
        let mover = world.spawn_empty("Mover", Transform::default());
        let script_path = write_script(
            "move_other.rhai",
            r#"
            let on_update = |dt, input| {
                world.find("Target").translate(1.0, 0.0, 0.0);
            };
            "#,
        );
        world.scripts.insert(mover, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, mover, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, mover, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert_eq!(world.transforms.get(target).unwrap().position.x, 1.0);
    }

    #[test]
    fn find_on_a_missing_name_returns_unit_rather_than_erroring() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Lonely", Transform::default());
        let script_path = write_script(
            "find_missing.rhai",
            r#"
            let on_update = |dt, input| {
                let missing = world.find("NoSuchEntity");
                if missing == () {
                    entity.x = 42.0;
                }
            };
            "#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert_eq!(world.transforms.get(entity).unwrap().position.x, 42.0);
    }

    #[test]
    fn cross_entity_reads_are_frozen_at_frame_start() {
        // A moves itself, then B reads A's position via find() in the same frame: B should see
        // A's position as of the start of the frame, not A's already-applied move.
        let mut world = World::new(test_camera());
        let a = world.spawn_empty("A", Transform::default());
        let b = world.spawn_empty("B", Transform::default());
        let a_script = write_script(
            "a_moves.rhai",
            r#"let on_update = |dt, input| { entity.translate(10.0, 0.0, 0.0); };"#,
        );
        let b_script = write_script(
            "b_reads_a.rhai",
            r#"let on_update = |dt, input| { entity.x = world.find("A").x; };"#,
        );
        world.scripts.insert(a, ScriptList(vec![ScriptAttachment { path: a_script.clone(), enabled: true }]));
        world.scripts.insert(b, ScriptList(vec![ScriptAttachment { path: b_script.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, a, 0, &a_script).expect("a should start");
        runtime.start_script(&world, b, 0, &b_script).expect("b should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, a, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        assert!(runtime.update_entity(&mut world, b, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert_eq!(world.transforms.get(a).unwrap().position.x, 10.0, "A should have moved");
        assert_eq!(
            world.transforms.get(b).unwrap().position.x,
            0.0,
            "B's read of A via find() should reflect A's position at the start of the frame, not A's move this same frame"
        );
    }

    #[test]
    fn on_update_despawns_another_entity() {
        let mut world = World::new(test_camera());
        let target = world.spawn_empty("Target", Transform::default());
        let killer = world.spawn_empty("Killer", Transform::default());
        let script_path = write_script(
            "despawn_other.rhai",
            r#"let on_update = |dt, input| { world.find("Target").despawn(); };"#,
        );
        world.scripts.insert(killer, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, killer, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, killer, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert!(!world.is_alive(target));
    }

    #[test]
    fn self_despawn_stops_remaining_attachments_this_frame() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("SelfDestruct", Transform::default());
        let despawn_script = write_script("despawn_self.rhai", r#"let on_update = |dt, input| { entity.despawn(); };"#);
        let mover_script = write_script(
            "mover_after_despawn.rhai",
            r#"let on_update = |dt, input| { entity.translate(1.0, 0.0, 0.0); };"#,
        );
        world.scripts.insert(
            entity,
            ScriptList(vec![
                ScriptAttachment { path: despawn_script.clone(), enabled: true },
                ScriptAttachment { path: mover_script.clone(), enabled: true },
            ]),
        );

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &despawn_script).expect("script 0 should start");
        runtime.start_script(&world, entity, 1, &mover_script).expect("script 1 should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert!(!world.is_alive(entity), "entity should be despawned");
    }

    #[test]
    fn world_spawn_returns_a_handle_usable_in_the_same_call() {
        let mut world = World::new(test_camera());
        let spawner = world.spawn_empty("Spawner", Transform::default());
        let script_path = write_script(
            "spawn_and_move.rhai",
            r#"
            let on_update = |dt, input| {
                let bullet = world.spawn_entity("Bullet", 1.0, 2.0, 3.0);
                bullet.translate(1.0, 0.0, 0.0);
            };
            "#,
        );
        world.scripts.insert(spawner, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, spawner, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, spawner, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        let spawned = world
            .iter_entities()
            .find(|&e| e != spawner && world.names.get(e).is_some_and(|n| n.0 == "Bullet"))
            .expect("a Bullet entity should have been spawned");
        let position = world.transforms.get(spawned).unwrap().position;
        assert!(
            position.abs_diff_eq(Vec3::new(2.0, 2.0, 3.0), 1e-5),
            "expected spawn position (1,2,3) plus a same-call translate(1,0,0), got {position:?}"
        );
    }

    #[test]
    fn on_update_renames_self_and_another_entity() {
        let mut world = World::new(test_camera());
        let other = world.spawn_empty("Old Name", Transform::default());
        let renamer = world.spawn_empty("Renamer", Transform::default());
        let script_path = write_script(
            "rename.rhai",
            r#"
            let on_update = |dt, input| {
                entity.set_name("New Renamer Name");
                world.find("Old Name").set_name("New Name");
            };
            "#,
        );
        world.scripts.insert(renamer, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, renamer, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let input = no_input();
        assert!(runtime.update_entity(&mut world, renamer, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert_eq!(world.names.get(renamer).unwrap().0, "New Renamer Name");
        assert_eq!(world.names.get(other).unwrap().0, "New Name");
    }

    #[test]
    fn attach_script_starts_running_within_the_same_session() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("LateBloomer", Transform::default());
        let mover_script = write_script("late_mover.rhai", r#"let on_update = |dt, input| { entity.translate(1.0, 0.0, 0.0); };"#);
        let attacher_script = write_script(
            "attacher.rhai",
            &format!(
                r#"let on_update = |dt, input| {{ entity.attach_script("{}"); }};"#,
                mover_script.display().to_string().replace('\\', "\\\\")
            ),
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: attacher_script.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &attacher_script).expect("attacher should start");
        runtime.begin_frame(&world);

        let input = no_input();
        // First frame: the attacher queues attach_script, which is applied (and started) during
        // this same update_entity call.
        assert!(runtime.update_entity(&mut world, entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        assert_eq!(
            world.scripts.get(entity).unwrap().0.len(),
            2,
            "the attached mover script should now be in the entity's ScriptList"
        );

        // Second frame: the newly-attached mover script should actually run now, not just sit
        // there inert until a hypothetical Stop/Play.
        runtime.begin_frame(&world);
        assert!(runtime.update_entity(&mut world, entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        assert!(
            world.transforms.get(entity).unwrap().position.x > 0.0,
            "the attached script should have moved the entity by its second frame"
        );
    }

    // `set_sprite`/`set_model` need a live `Renderer` (a real GPU device) to actually load
    // anything, which a unit test has no portable way to stand up — see
    // `world::tests::clone_is_independent_of_the_original`'s doc comment for the same
    // constraint. What *is* testable without one is the graceful-failure path (`renderer: None`
    // reports a `ScriptError` instead of panicking or silently doing nothing) and
    // `detach_renderable`, which touches no GPU state at all.

    #[test]
    fn set_sprite_without_a_renderer_reports_an_error_instead_of_panicking() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("NeedsASprite", Transform::default());
        let script_path = write_script(
            "set_sprite.rhai",
            r#"let on_update = |dt, input| { entity.set_sprite("assets/textures/missing.png"); };"#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let errors = runtime.update_entity(&mut world, entity, 0.1, &no_input(), &no_mouse(), (0.0, 0.0), None);
        assert_eq!(errors.len(), 1, "expected one error reporting the missing renderer, got {errors:?}");
        assert!(world.renderables.get(entity).is_none(), "nothing should have been attached");
    }

    #[test]
    fn set_model_without_a_renderer_reports_an_error_instead_of_panicking() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("NeedsAModel", Transform::default());
        let script_path = write_script(
            "set_model.rhai",
            r#"let on_update = |dt, input| { entity.set_model("assets/models/missing.obj"); };"#,
        );
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        let errors = runtime.update_entity(&mut world, entity, 0.1, &no_input(), &no_mouse(), (0.0, 0.0), None);
        assert_eq!(errors.len(), 1, "expected one error reporting the missing renderer, got {errors:?}");
        assert!(world.renderables.get(entity).is_none(), "nothing should have been attached");
    }

    #[test]
    fn detach_renderable_needs_no_renderer() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("MaybeVisible", Transform::default());
        let script_path = write_script("detach.rhai", r#"let on_update = |dt, input| { entity.detach_renderable(); };"#);
        world.scripts.insert(entity, ScriptList(vec![ScriptAttachment { path: script_path.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, entity, 0, &script_path).expect("script should start");
        runtime.begin_frame(&world);

        // No renderable was ever attached, so this also doubles as a no-op-if-absent check —
        // the point is that it doesn't error or panic for lack of a renderer, unlike set_sprite/
        // set_model above.
        assert!(runtime.update_entity(&mut world, entity, 0.1, &no_input(), &no_mouse(), (0.0, 0.0), None).is_empty());
        assert!(world.renderables.get(entity).is_none());
    }

    #[test]
    fn scene_change_tears_down_non_persistent_entities_but_keeps_persistent_ones_running() {
        use crate::scene_file::{EntityRecord, SceneFile};

        let mut world = World::new(test_camera());
        let persistent_entity = world.spawn_empty("Persistent", Transform::default());
        let transient_entity = world.spawn_empty("Transient", Transform::default());
        let trigger_entity = world.spawn_empty("Trigger", Transform::default());

        // Marks itself persistent from `on_start`, then counts frames via `on_update` — the
        // counter is the script's own Rhai-local state, which should survive the scene change
        // untouched (not reset/restarted) if persistence works as intended.
        let persistent_script = write_script(
            "persistent.rhai",
            r#"
            let counter = 0.0;
            let on_start = || { entity.set_persistent(true); };
            let on_update = |dt, input| {
                counter += 1.0;
                entity.x = counter;
            };
            "#,
        );
        world.scripts.insert(
            persistent_entity,
            ScriptList(vec![ScriptAttachment { path: persistent_script.clone(), enabled: true }]),
        );

        // The entity the new scene spawns; its `on_start` marks it, so we can confirm it actually
        // started (rather than just being inert data copied into `World`).
        let new_entity_script = write_script(
            "new_entity.rhai",
            r#"let on_start = || { entity.set_name("Started"); };"#,
        );
        let target_scene = write_scene(
            "target.ron",
            &SceneFile {
                entities: vec![EntityRecord {
                    name: "NewEntity".to_string(),
                    transform: Transform::default(),
                    renderable: None,
                    scripts: vec![ScriptAttachment { path: new_entity_script.clone(), enabled: true }],
                    camera: None,
                }],
            },
        );

        let trigger_script = write_script(
            "trigger.rhai",
            &format!(
                r#"let on_update = |dt, input| {{ scene.change("{}"); }};"#,
                target_scene.display().to_string().replace('\\', "\\\\")
            ),
        );
        world.scripts.insert(trigger_entity, ScriptList(vec![ScriptAttachment { path: trigger_script.clone(), enabled: true }]));

        let mut runtime = test_runtime();
        runtime.start_script(&world, persistent_entity, 0, &persistent_script).expect("persistent script should start");
        runtime.start_script(&world, trigger_entity, 0, &trigger_script).expect("trigger script should start");
        runtime.begin_frame(&world);

        let input = no_input();

        // First frame: applies the queued `set_persistent(true)` from `on_start` (queued commands
        // apply one frame later — see `start_script`'s doc comment) and advances the counter to 1.
        assert!(runtime.update_entity(&mut world, persistent_entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        assert!(world.is_persistent(persistent_entity), "entity.set_persistent(true) from on_start should have applied");

        // Trigger the scene change. The trigger entity itself is non-persistent, so it (and the
        // transient entity) should be torn down along with the rest of the outgoing scene.
        assert!(runtime.update_entity(&mut world, trigger_entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());

        assert!(!world.is_alive(transient_entity), "non-persistent entities should be despawned by the scene change");
        assert!(!world.is_alive(trigger_entity), "the entity that triggered the change is itself non-persistent");
        assert!(world.is_alive(persistent_entity), "a persistent entity should survive the scene change");

        let new_entity = world
            .iter_entities()
            .find(|&e| world.names.get(e).is_some_and(|n| n.0 == "NewEntity"))
            .expect("the new scene's entity should be spawned");
        assert_ne!(new_entity, persistent_entity);

        // Second frame, post-change: the persistent entity's script keeps running from where it
        // left off rather than being restarted (which would reset `counter` back to 0/1). This
        // same `update_entity` call's `drain_commands` also flushes the new entity's `on_start`-
        // queued `set_name` from the scene-change frame (queued commands apply one frame later —
        // see `start_script`'s doc comment), so it doubles as confirmation `on_start` actually ran.
        assert!(runtime.update_entity(&mut world, persistent_entity, 0.1, &input, &no_mouse(), (0.0, 0.0), None).is_empty());
        assert_eq!(
            world.transforms.get(persistent_entity).unwrap().position.x,
            2.0,
            "persistent entity's script state should continue across the scene change, not reset"
        );
        assert_eq!(
            world.names.get(new_entity).unwrap().0,
            "Started",
            "the new scene's entity should have run its on_start"
        );
    }
}
