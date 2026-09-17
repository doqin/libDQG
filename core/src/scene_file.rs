use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::renderer::{Model, Renderer, Sprite, Texture};
use crate::scripting::ScriptAttachment;
use crate::world::{CameraComponent, Renderable, Transform};

/// The on-disk form of a scene: an editor project's `scenes/main.ron`, or an exported game's
/// `res/scene.ron` — both the editor and the `runtime` player deserialize this same type.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct SceneFile {
    pub entities: Vec<EntityRecord>,
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
                    // No saved/explicit size — this is the placeholder `attach_renderable`
                    // probes with before it knows the real one.
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
/// feature and read by the `runtime` binary at startup, alongside `res/scene.ron`.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GameManifest {
    pub title: String,
    pub width: u32,
    pub height: u32,
}
