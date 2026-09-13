use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::camera::Camera;
use crate::ecs::entity::EntityAllocator;
use crate::ecs::{ComponentStore, Entity};
use crate::renderer::{Model, Sprite};
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

pub struct Name(pub String);

/// What an entity draws as. An entity is a sprite or a model, never both, so this is one enum
/// rather than two parallel component stores that would otherwise need to be kept in sync.
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

/// A spatial container of entities, distinct from [`crate::scene::Scene`] (which means
/// "game state/screen" in this framework, not a place). Each entity is a [`Transform`] plus a
/// [`Renderable`] and an optional [`Name`].
pub struct World {
    allocator: EntityAllocator,
    pub transforms: ComponentStore<Transform>,
    pub names: ComponentStore<Name>,
    pub renderables: ComponentStore<Renderable>,
    pub camera: Camera,
}

impl World {
    pub fn new(camera: Camera) -> Self {
        Self {
            allocator: EntityAllocator::new(),
            transforms: ComponentStore::new(),
            names: ComponentStore::new(),
            renderables: ComponentStore::new(),
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

    pub fn despawn(&mut self, entity: Entity) {
        self.allocator.despawn(entity);
        self.names.remove(entity);
        self.transforms.remove(entity);
        self.renderables.remove(entity);
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
}
