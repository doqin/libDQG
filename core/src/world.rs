use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::camera::Camera;
use crate::ecs::entity::EntityAllocator;
use crate::ecs::{ComponentStore, Entity};
use crate::renderer::{Model, Sprite};
use crate::scripting::ScriptList;
use crate::transform::Transformable;

/// Position/rotation/scale, kept as separate fields (rather than a raw [`Mat4`]) so an editor
/// inspector can show and edit them independently. The ECS component is the source of truth;
/// [`Renderable::sync_transform`] copies it into the actual GPU-facing [`Sprite`]/[`Model`]
/// matrix each frame via [`Transformable::set_transform`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }
}

impl Transform {
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

#[derive(Clone)]
pub struct Name(pub String);

/// What an entity draws as. An entity is a sprite or a model, never both, so this is one enum
/// rather than two parallel component stores that would otherwise need to be kept in sync.
#[derive(Clone)]
pub enum Renderable {
    Sprite(Sprite),
    Model(Model),
}

impl Renderable {
    /// Writes `transform` into the underlying [`Sprite`]/[`Model`]'s own matrix, overwriting any
    /// transform accumulated on it directly. Call once per entity per frame before drawing.
    pub fn sync_transform(&mut self, transform: &Transform) {
        let matrix = transform.to_mat4();
        match self {
            Renderable::Sprite(sprite) => {
                sprite.set_transform(matrix);
            }
            Renderable::Model(model) => {
                model.set_transform(matrix);
            }
        }
    }

    /// Bounding-sphere radius in local (pre-transform) space, for picking.
    pub fn bounding_radius(&self) -> f32 {
        match self {
            Renderable::Sprite(sprite) => 0.5 * sprite.width.hypot(sprite.height),
            Renderable::Model(model) => model.bounding_radius,
        }
    }

    /// Bounding-sphere center in local (pre-transform) space, for picking.
    pub fn bounding_center(&self) -> Vec3 {
        match self {
            Renderable::Sprite(sprite) => Vec3::new(sprite.width * 0.5, sprite.height * 0.5, 0.0),
            Renderable::Model(model) => model.bounding_center,
        }
    }
}

/// A camera lens attached to an entity. `active` marks the single camera entity Play mode should
/// look through — a bool flag rather than a separate "active camera entity" reference, since
/// entities aren't referenced by stable ID across a scene save/load (scripts look entities up by
/// name for the same reason). The UI is responsible for keeping at most one entity's flag set
/// (radio-button semantics, see `editor::draw_camera_editor`); `World::active_camera` defends
/// against more than one somehow being set by just using the first it finds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CameraComponent {
    /// Vertical field of view, in degrees — same convention as [`Camera::fov`].
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
    pub active: bool,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self { fov: 45.0, znear: 0.1, zfar: 100.0, active: false }
    }
}

impl CameraComponent {
    /// Looks down local -Z, rotated by `transform.rotation` into world space, then converted to
    /// the yaw/pitch [`Camera`] stores via [`Camera::yaw_pitch_from_forward`] — any roll in
    /// `transform.rotation` is dropped, since the FPS-style `Camera` has no roll to express it
    /// with. `aspect` is viewport-derived, not stored on the component (mirrors how
    /// `Renderer::resize` keeps `Camera::aspect` current rather than baking it into camera data).
    pub fn derive_camera(&self, transform: &Transform, aspect: f32) -> Camera {
        let forward = transform.rotation * Vec3::NEG_Z;
        let (yaw, pitch) = Camera::yaw_pitch_from_forward(forward);
        Camera {
            position: transform.position,
            yaw,
            pitch,
            aspect,
            fov: self.fov,
            znear: self.znear,
            zfar: self.zfar,
        }
    }
}

/// A spatial container of entities, distinct from [`crate::scene::Scene`] (which means
/// "game state/screen" in this framework, not a place). Each entity is a [`Transform`] plus a
/// [`Renderable`] and an optional [`Name`].
///
/// `Clone` is cheap and GPU-allocation-free: a cloned [`Renderable`]'s `wgpu::Buffer`/
/// `wgpu::BindGroup` handles are refcount copies of the same GPU resource, not duplicates (wgpu
/// itself derives `Clone` for both). The editor relies on this to snapshot the whole `World`
/// before entering Play and restore it wholesale on Stop.
#[derive(Clone)]
pub struct World {
    allocator: EntityAllocator,
    pub transforms: ComponentStore<Transform>,
    pub names: ComponentStore<Name>,
    pub renderables: ComponentStore<Renderable>,
    pub scripts: ComponentStore<ScriptList>,
    pub cameras: ComponentStore<CameraComponent>,
    pub camera: Camera,
}

impl World {
    pub fn new(camera: Camera) -> Self {
        Self {
            allocator: EntityAllocator::new(),
            transforms: ComponentStore::new(),
            names: ComponentStore::new(),
            renderables: ComponentStore::new(),
            scripts: ComponentStore::new(),
            cameras: ComponentStore::new(),
            camera,
        }
    }

    pub fn spawn_empty(&mut self, name: impl Into<String>, transform: Transform) -> Entity {
        let entity = self.allocator.spawn();
        self.names.insert(entity, Name(name.into()));
        self.transforms.insert(entity, transform);
        entity
    }

    pub fn set_renderable(&mut self, entity: Entity, renderable: Renderable) {
        self.renderables.insert(entity, renderable);
    }

    pub fn clear_renderable(&mut self, entity: Entity) {
        self.renderables.remove(entity);
    }

    pub fn set_camera(&mut self, entity: Entity, component: CameraComponent) {
        self.cameras.insert(entity, component);
    }

    pub fn clear_camera(&mut self, entity: Entity) {
        self.cameras.remove(entity);
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.allocator.despawn(entity);
        self.names.remove(entity);
        self.transforms.remove(entity);
        self.renderables.remove(entity);
        self.scripts.remove(entity);
        self.cameras.remove(entity);
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.allocator.is_alive(entity)
    }

    /// Every live entity (every entity has a [`Transform`], so iterating that store enumerates
    /// them all — unlike `renderables`, which only some entities have).
    pub fn iter_entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.transforms.iter().map(|(entity, _)| entity)
    }

    /// Syncs every entity's ECS [`Transform`] into its [`Renderable`]'s own GPU-facing matrix.
    /// Call once per frame before drawing.
    pub fn sync_transforms(&mut self) {
        for (entity, renderable) in self.renderables.iter_mut() {
            if let Some(transform) = self.transforms.get(entity) {
                renderable.sync_transform(transform);
            }
        }
    }

    /// The derived [`Camera`] for whichever entity has [`CameraComponent::active`] set, if any —
    /// what Play mode pushes to the renderer in place of the edit-mode fly camera. `aspect` is
    /// threaded through rather than read off `self.camera` so the caller can supply the current
    /// viewport's aspect the same frame it's computed.
    ///
    /// Returns `None` if no entity is marked active (or the active entity's `Transform` is
    /// somehow missing) — callers should fall back to whatever camera they'd otherwise use.
    pub fn active_camera(&self, aspect: f32) -> Option<Camera> {
        let (entity, component) = self.cameras.iter().find(|(_, c)| c.active)?;
        let transform = self.transforms.get(entity)?;
        Some(component.derive_camera(transform, aspect))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::{ScriptAttachment, ScriptList};
    use std::path::PathBuf;

    fn test_camera() -> Camera {
        Camera { position: Vec3::ZERO, yaw: 0.0, pitch: 0.0, aspect: 1.0, fov: 45.0, znear: 0.1, zfar: 100.0 }
    }

    #[test]
    fn despawn_clears_attached_scripts() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Scripted", Transform::default());
        world.scripts.insert(
            entity,
            ScriptList(vec![ScriptAttachment { path: PathBuf::from("scripts/move.rhai"), enabled: true }]),
        );

        world.despawn(entity);

        assert!(world.scripts.get(entity).is_none());
    }

    #[test]
    fn despawn_clears_attached_camera() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Cam", Transform::default());
        world.set_camera(entity, CameraComponent::default());

        world.despawn(entity);

        assert!(world.cameras.get(entity).is_none());
    }

    #[test]
    fn derive_camera_looks_down_local_negative_z() {
        let component = CameraComponent::default();
        let camera = component.derive_camera(&Transform::default(), 1.0);

        assert!(camera.forward().abs_diff_eq(Vec3::NEG_Z, 1e-5));
    }

    /// Locks the rotation convention `CameraComponent::derive_camera` uses so a future refactor
    /// can't silently flip which way a camera entity faces.
    #[test]
    fn derive_camera_respects_rotation() {
        let component = CameraComponent::default();
        let transform = Transform { rotation: Quat::from_rotation_y(90.0_f32.to_radians()), ..Transform::default() };

        let camera = component.derive_camera(&transform, 1.0);

        assert!(camera.forward().abs_diff_eq(Vec3::NEG_X, 1e-4));
    }

    #[test]
    fn active_camera_returns_none_when_nothing_is_marked_active() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Cam", Transform::default());
        world.set_camera(entity, CameraComponent { active: false, ..CameraComponent::default() });

        assert!(world.active_camera(1.0).is_none());
    }

    #[test]
    fn active_camera_derives_from_the_marked_entity() {
        let mut world = World::new(test_camera());
        let position = Vec3::new(1.0, 2.0, 3.0);
        let entity = world.spawn_empty("Cam", Transform { position, ..Transform::default() });
        world.set_camera(entity, CameraComponent { fov: 60.0, znear: 0.5, zfar: 50.0, active: true });

        let camera = world.active_camera(1.7).expect("an active camera entity exists");

        assert_eq!(camera.position, position);
        assert_eq!(camera.fov, 60.0);
        assert_eq!(camera.znear, 0.5);
        assert_eq!(camera.zfar, 50.0);
        assert_eq!(camera.aspect, 1.7);
    }

    /// Guards the editor's Play/Stop snapshot-and-restore, which relies on `World::clone()`
    /// producing an independent copy (see [`World`]'s doc comment) — a future change that made
    /// some field shallow/shared (e.g. wrapping a component store in an `Rc` for some other
    /// reason) would silently break Stop's revert-to-pre-Play guarantee without this test
    /// catching it. Doesn't exercise a [`Renderable`] directly (that needs a live `Renderer`/GPU
    /// device to construct, impractical in a unit test), but `Renderable`'s own GPU-backed
    /// fields are cheap handle clones by construction — `wgpu::Buffer`/`BindGroup`/`Texture` all
    /// derive `Clone` themselves — so cloning one just copies a handle, never GPU state; what
    /// this test guards is the plain-data component stores instead.
    #[test]
    fn clone_is_independent_of_the_original() {
        let mut world = World::new(test_camera());
        let entity = world.spawn_empty("Original", Transform { position: Vec3::new(1.0, 2.0, 3.0), ..Transform::default() });
        world.scripts.insert(
            entity,
            ScriptList(vec![ScriptAttachment { path: PathBuf::from("scripts/move.rhai"), enabled: true }]),
        );

        let mut clone = world.clone();

        // Mutate the clone in every way Play can mutate a World: move, rename, despawn, and
        // spawn a brand new entity.
        clone.transforms.get_mut(entity).unwrap().position = Vec3::ZERO;
        clone.names.get_mut(entity).unwrap().0 = "Renamed".to_string();
        clone.scripts.get_mut(entity).unwrap().0[0].enabled = false;
        let new_entity = clone.spawn_empty("SpawnedDuringPlay", Transform::default());

        assert_eq!(world.transforms.get(entity).unwrap().position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(world.names.get(entity).unwrap().0, "Original");
        assert!(world.scripts.get(entity).unwrap().0[0].enabled);
        assert!(!world.is_alive(new_entity), "an entity spawned only on the clone shouldn't exist on the original");
    }
}
