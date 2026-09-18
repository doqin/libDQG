use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ecs::Entity;
use crate::renderer::{Model, Renderer, Sprite, Texture};
use crate::scripting::{ScriptAttachment, ScriptList};
use crate::world::{CameraComponent, Renderable, Transform, World};

/// The on-disk form of a scene: one of an editor project's `scenes/*.ron` files, or one of an
/// exported game's `res/scenes/*.ron` files — both the editor and the `runtime` player
/// deserialize this same type. A project can hold more than one (see
/// `editor::Project::list_scenes`); which one a game boots into is
/// [`GameManifest::start_scene`], and scripts switch between them via `scene.change(path)`
/// (see `crate::scripting`'s `ScriptScene`/`WorldCommand::ChangeScene`).
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct SceneFile {
    pub entities: Vec<EntityRecord>,
}

impl SceneFile {
    /// Reads and parses the `.ron` scene file at `path` — the single source of truth for the
    /// on-disk format, used by the editor, `runtime`, and `ScriptRuntime`'s `scene.change(...)`
    /// handling alike.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Ok(ron::from_str(&fs::read_to_string(path)?)?)
    }

    /// Serializes and writes `self` to `path` as pretty-printed RON — the save-side counterpart
    /// to [`SceneFile::load`].
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        fs::write(path, ron::ser::to_string_pretty(self, Default::default())?)?;
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntityRecord {
    pub name: String,
    pub transform: Transform,
    pub renderable: Option<RenderableAsset>,
    #[serde(default)]
    pub scripts: Vec<ScriptAttachment>,
    #[serde(default)]
    pub camera: Option<CameraComponent>,
}

impl EntityRecord {
    /// Spawns this record into `world`: creates the entity with its saved name/transform, loads
    /// its `renderable` (if any, resolved against `base_dir` — an editor project's root, or an
    /// exported game's `res/` directory) via [`RenderableAsset::load`], and copies over its
    /// `scripts`/`camera` components verbatim (starting the scripts is a separate step — see
    /// `crate::scripting::ScriptRuntime::start_all_scripts`). A renderable load failure is
    /// reported to stderr and leaves the entity without one, rather than failing the whole spawn.
    /// Returns the spawned entity plus the [`RenderableAsset`] if one loaded successfully — the
    /// editor's asset-tracking map needs it back for re-saving; other callers can ignore it.
    pub fn spawn_into(self, world: &mut World, renderer: Option<&Renderer>, base_dir: &Path) -> (Entity, Option<RenderableAsset>) {
        let entity = world.spawn_empty(self.name, self.transform);
        let mut loaded_asset = None;

        if let Some(asset) = self.renderable {
            match renderer {
                Some(renderer) => match asset.load(renderer, base_dir) {
                    Ok(renderable) => {
                        world.set_renderable(entity, renderable);
                        loaded_asset = Some(asset);
                    }
                    Err(e) => eprintln!("Failed to load renderable: {e}"),
                },
                None => eprintln!("Failed to load renderable: no renderer available"),
            }
        }
        if !self.scripts.is_empty() {
            world.scripts.insert(entity, ScriptList(self.scripts));
        }
        if let Some(component) = self.camera {
            world.set_camera(entity, component);
        }

        (entity, loaded_asset)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderableKind {
    Sprite,
    Model,
}

/// The serializable stand-in for a [`Renderable`]: just the source asset path(s) (relative to
/// the project root, or an exported game's `res/` directory) plus the handful of params needed
/// to reconstruct it, since the runtime `Renderable` itself holds live GPU resources that can't
/// be serialized.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum RenderableAsset {
    Sprite { texture_path: PathBuf, width: f32, height: f32 },
    Model { model_path: PathBuf },
}

impl RenderableAsset {
    /// `base_dir` resolves the stored relative asset path — an editor project's root when called
    /// from the editor, or an exported game's `res/` directory when called from `runtime`.
    pub fn load(&self, renderer: &Renderer, base_dir: &Path) -> anyhow::Result<Renderable> {
        match self {
            RenderableAsset::Sprite { texture_path, width, height } => {
                let path = base_dir.join(texture_path);
                let texture = Arc::new(
                    Texture::from_path(renderer, &path).map_err(|e| anyhow::anyhow!(e))?,
                );
                let mut sprite = Sprite::new(texture);
                if *width > 0.0 && *height > 0.0 {
                    sprite.width = *width;
                    sprite.height = *height;
                } else {
                    // No saved/explicit size: fall back to a unit-square fit. Covers both the
                    // editor's `attach_renderable` probe (before it knows the real size) and any
                    // other zero-sized `width`/`height` this shared loader is handed.
                    sprite.fit_within_unit_square();
                }
                Ok(Renderable::Sprite(sprite))
            }
            RenderableAsset::Model { model_path } => {
                let path = base_dir.join(model_path);
                let model = Model::load(renderer, &path)?;
                Ok(Renderable::Model(model))
            }
        }
    }

    pub fn kind(&self) -> RenderableKind {
        match self {
            RenderableAsset::Sprite { .. } => RenderableKind::Sprite,
            RenderableAsset::Model { .. } => RenderableKind::Model,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            RenderableAsset::Sprite { texture_path, .. } => texture_path,
            RenderableAsset::Model { model_path, .. } => model_path,
        }
    }
}

/// Window settings for an exported game: written to `res/game.ron` by the editor's export
/// feature and read by the `runtime` binary at startup, alongside `res/scenes/*.ron`.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameManifest {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Project-relative path (mirrored 1:1 under `res/`) to the scene `runtime` boots into —
    /// e.g. `"scenes/main.ron"`, resolved as `res_dir.join(start_scene)`.
    pub start_scene: PathBuf,
}
