use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libdqg::ecs::Entity;
use libdqg::renderer::{Model, Renderer, Sprite, Texture};
use libdqg::world::{Renderable, Transform, World};

const MANIFEST_FILE: &str = "project.ron";
const SCENE_FILE: &str = "scenes/main.ron";

/// A project on disk: a folder containing a manifest plus `assets/`, `scenes/`, `scripts/`
/// subfolders. `scripts/` is reserved for a future scripting system — nothing reads or writes it
/// yet.
pub struct Project {
    pub root: PathBuf,
    pub manifest: ProjectManifest,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct SceneFile {
    pub entities: Vec<EntityRecord>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntityRecord {
    pub name: String,
    pub transform: Transform,
    pub renderable: Option<RenderableAsset>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderableKind {
    Sprite,
    Model,
}

/// The serializable stand-in for a [`Renderable`]: just the source asset path(s) (relative to
/// the project root) plus the handful of params needed to reconstruct it, since the runtime
/// `Renderable` itself holds live GPU resources that can't be serialized.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum RenderableAsset {
    Sprite { texture_path: PathBuf, width: f32, height: f32 },
    Model { model_path: PathBuf },
}

impl RenderableAsset {
    pub fn load(&self, renderer: &Renderer, project: &Project) -> anyhow::Result<Renderable> {
        match self {
            RenderableAsset::Sprite { texture_path, width, height } => {
                let path = project.root.join(texture_path);
                let texture = Arc::new(
                    Texture::from_path(renderer, &path).map_err(|e| anyhow::anyhow!(e))?,
                );
                let mut sprite = Sprite::new(texture);
                sprite.width = *width;
                sprite.height = *height;
                Ok(Renderable::Sprite(sprite))
            }
            RenderableAsset::Model { model_path } => {
                let path = project.root.join(model_path);
                let model = Model::load(renderer, &path)?;
                Ok(Renderable::Model(model))
            }
        }
    }
}

impl Project {
    fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn textures_dir(&self) -> PathBuf {
        self.assets_dir().join("textures")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.assets_dir().join("models")
    }

    fn scripts_dir(&self) -> PathBuf {
        self.root.join("scripts")
    }

    fn scenes_dir(&self) -> PathBuf {
        self.root.join("scenes")
    }

    fn scene_path(&self) -> PathBuf {
        self.root.join(SCENE_FILE)
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    /// Creates a new project folder at `root`: `assets/{textures,models}/`, `scenes/`,
    /// `scripts/`, a manifest, and an empty scene file so [`Project::open`]/[`Project::load_scene`]
    /// always find a valid file even before the first save.
    pub fn create(root: PathBuf) -> anyhow::Result<Self> {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled Project".to_string());

        let project = Project { root, manifest: ProjectManifest { format_version: 1, name } };

        fs::create_dir_all(project.textures_dir())?;
        fs::create_dir_all(project.models_dir())?;
        fs::create_dir_all(project.scenes_dir())?;
        fs::create_dir_all(project.scripts_dir())?;

        fs::write(project.manifest_path(), ron::ser::to_string_pretty(&project.manifest, Default::default())?)?;
        fs::write(project.scene_path(), ron::ser::to_string_pretty(&SceneFile::default(), Default::default())?)?;

        Ok(project)
    }

    /// Opens an existing project folder by reading its manifest.
    pub fn open(root: PathBuf) -> anyhow::Result<Self> {
        let manifest_path = root.join(MANIFEST_FILE);
        let manifest: ProjectManifest = ron::from_str(&fs::read_to_string(manifest_path)?)?;
        Ok(Project { root, manifest })
    }

    pub fn save_scene(&self, world: &World, assets: &HashMap<Entity, RenderableAsset>) -> anyhow::Result<()> {
        let entities = world
            .iter_entities()
            .map(|entity| EntityRecord {
                name: world.names.get(entity).map(|n| n.0.clone()).unwrap_or_default(),
                transform: *world.transforms.get(entity).unwrap(),
                renderable: assets.get(&entity).cloned(),
            })
            .collect();

        let scene_file = SceneFile { entities };
        fs::write(self.scene_path(), ron::ser::to_string_pretty(&scene_file, Default::default())?)?;
        Ok(())
    }

    pub fn load_scene(&self) -> anyhow::Result<SceneFile> {
        Ok(ron::from_str(&fs::read_to_string(self.scene_path())?)?)
    }

    /// Lists assets of the given kind already in the project (relative to the project root),
    /// for the Assets panel and the inspector's attach-renderable picker to share.
    pub fn list_assets(&self, kind: RenderableKind) -> Vec<PathBuf> {
        let dir = match kind {
            RenderableKind::Sprite => self.textures_dir(),
            RenderableKind::Model => self.models_dir(),
        };

        let Ok(read_dir) = fs::read_dir(&dir) else { return Vec::new() };
        let mut paths: Vec<PathBuf> = read_dir
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| path.strip_prefix(&self.root).map(Path::to_path_buf).ok())
            .collect();
        paths.sort();
        paths
    }

    /// Copies `src` into the project's `assets/` folder, sorted into `textures/`/`models/` by
    /// extension (anything else falls back to a flat `assets/` folder). Importing an `.obj`
    /// also copies its referenced `.mtl` file(s) and the textures those reference, so the model
    /// still loads from its new location. Returns the new path, relative to the project root.
    pub fn import_asset(&self, src: &Path) -> anyhow::Result<PathBuf> {
        let extension = src.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
        let dest_dir = match extension.as_str() {
            "png" | "jpg" | "jpeg" => self.textures_dir(),
            "obj" | "mtl" => self.models_dir(),
            _ => self.assets_dir(),
        };
        fs::create_dir_all(&dest_dir)?;

        let file_name = src.file_name().ok_or_else(|| anyhow::anyhow!("import source has no file name"))?;
        let dest = dest_dir.join(file_name);
        fs::copy(src, &dest)?;

        if extension == "obj" {
            Self::import_obj_dependencies(src, &dest_dir)?;
        }

        Ok(dest.strip_prefix(&self.root)?.to_path_buf())
    }

    /// Best-effort copy of an OBJ's `mtllib` material file(s) and the textures they reference
    /// (`map_Kd`/`map_Ka`/`map_Ks`/`bump`/... lines), alongside it in `dest_dir`, mirroring
    /// [`Model::load`]'s own path resolution (relative to the OBJ's/MTL's own directory) so the
    /// copied files still resolve once loaded from the project. Not a full MTL parser — assumes
    /// the referenced filename is the last whitespace-separated token on the line, which covers
    /// typical exporter output.
    fn import_obj_dependencies(obj_src: &Path, dest_dir: &Path) -> anyhow::Result<()> {
        let obj_dir = obj_src.parent().unwrap_or_else(|| Path::new(""));
        let obj_text = fs::read_to_string(obj_src)?;

        for mtl_name in obj_text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("mtllib"))
            .flat_map(|rest| rest.split_whitespace())
        {
            let mtl_src = obj_dir.join(mtl_name);
            if !mtl_src.is_file() {
                continue;
            }
            fs::copy(&mtl_src, dest_dir.join(mtl_name))?;

            let mtl_dir = mtl_src.parent().unwrap_or_else(|| Path::new(""));
            let mtl_text = fs::read_to_string(&mtl_src)?;
            for line in mtl_text.lines() {
                let mut tokens = line.trim().split_whitespace();
                let Some(directive) = tokens.next() else { continue };
                let is_texture_map =
                    directive.starts_with("map_") || matches!(directive, "bump" | "disp" | "decal");
                if !is_texture_map {
                    continue;
                }
                let Some(texture_name) = tokens.last() else { continue };

                let texture_src = mtl_dir.join(texture_name);
                if !texture_src.is_file() {
                    continue;
                }
                let texture_dest = dest_dir.join(texture_name);
                if let Some(parent) = texture_dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&texture_src, &texture_dest)?;
            }
        }

        Ok(())
    }
}
