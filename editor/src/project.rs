use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use libdqg::ecs::Entity;
use libdqg::world::World;

/// [`SceneFile`]/[`EntityRecord`]/[`RenderableAsset`]/[`RenderableKind`] live in `libdqg::scene_file`
/// so the standalone `runtime` player crate can deserialize the exact same scene format without
/// depending on this (egui/rfd-heavy) editor crate — see the export/compile implementation plan.
pub use libdqg::scene_file::{EntityRecord, RenderableAsset, RenderableKind, SceneFile};

pub const MANIFEST_FILE: &str = "project.ron";
const DEFAULT_SCENE_FILE: &str = "scenes/main.ron";

fn default_start_scene() -> PathBuf {
    PathBuf::from(DEFAULT_SCENE_FILE)
}

/// Rewrites `path`'s separators to `/`, regardless of the host OS's own convention. Every
/// project-relative path this module hands back or persists (`list_scenes`/`list_scripts`/
/// `list_assets`'s `strip_prefix` results, `create_scene`/`create_script`/`import_asset`'s
/// returned paths, `set_start_scene`'s argument) must round-trip identically whether the project
/// is saved on Windows and later opened/exported-and-run on Linux (`runtime`, CI) or vice versa.
/// `Path::join` and component parsing accept `/` as a separator on Windows too, but Unix treats a
/// literal `\` as an ordinary filename character, not a separator — so a path built with native
/// separators on Windows (`scenes\main.ron`) silently fails to resolve at all once read back on
/// Linux. Rebuilding the `PathBuf` from the normalized string (rather than via `.join(...)`,
/// which would reintroduce native separators) is what makes it stick: `Path`'s `Display`/
/// `to_str()` never rewrites an already-`/`-separated string on Windows, only `.join()` inserts a
/// native separator between components.
fn to_portable_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

/// Starter content for [`Project::create_script`]'s "New Script" boilerplate — both hooks
/// [`libdqg::scripting::ScriptRuntime`] looks for, stubbed out. `on_start`/`on_update` must stay
/// `let`-bound closures rather than plain `fn`s (see `ScriptRuntime::start_script`'s doc comment
/// on why) — this template exists partly so a new script starts from a working example of that,
/// not just a blank file.
const SCRIPT_BOILERPLATE: &str = r#"// Called once when this script starts (Play begins, or the script is attached mid-Play).
let on_start = || {
};

// Called every frame while playing.
// `dt` is the elapsed time in seconds since the last frame.
// `input` lets you check keys, e.g. input.is_held("KeyW") or input.is_pressed("Space").
let on_update = |dt, input| {
};
"#;

/// A project on disk: a folder containing a manifest plus `assets/`, `scenes/`, `scripts/`
/// subfolders. Entities can attach one or more `.rhai` files from `scripts/` — see
/// [`Project::list_scripts`]/[`Project::create_script`] and
/// [`libdqg::scripting::ScriptRuntime`].
pub struct Project {
    pub root: PathBuf,
    pub manifest: ProjectManifest,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub name: String,
    /// Project-relative path to the scene a new `EditorScene`/export boots into — e.g.
    /// `"scenes/main.ron"`. `#[serde(default)]`d since projects saved before multi-scene support
    /// predate this field.
    #[serde(default = "default_start_scene")]
    pub start_scene: PathBuf,
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

    pub fn scripts_dir(&self) -> PathBuf {
        self.root.join("scripts")
    }

    fn scenes_dir(&self) -> PathBuf {
        self.root.join("scenes")
    }

    /// The absolute path to the project's start scene — see [`ProjectManifest::start_scene`].
    pub fn start_scene_path(&self) -> PathBuf {
        self.root.join(&self.manifest.start_scene)
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

        let project = Project {
            root,
            manifest: ProjectManifest { format_version: 1, name, start_scene: PathBuf::from(DEFAULT_SCENE_FILE) },
        };

        fs::create_dir_all(project.textures_dir())?;
        fs::create_dir_all(project.models_dir())?;
        fs::create_dir_all(project.scenes_dir())?;
        fs::create_dir_all(project.scripts_dir())?;

        fs::write(project.manifest_path(), ron::ser::to_string_pretty(&project.manifest, Default::default())?)?;
        SceneFile::default().save(&project.start_scene_path())?;

        Ok(project)
    }

    /// Opens an existing project folder by reading its manifest.
    pub fn open(root: PathBuf) -> anyhow::Result<Self> {
        let manifest_path = root.join(MANIFEST_FILE);
        let manifest: ProjectManifest = ron::from_str(&fs::read_to_string(manifest_path)?)?;
        Ok(Project { root, manifest })
    }

    /// Saves `world` to the scene file at `path` (project-relative).
    pub fn save_scene(&self, path: &Path, world: &World, assets: &HashMap<Entity, RenderableAsset>) -> anyhow::Result<()> {
        let entities = world
            .iter_entities()
            .map(|entity| EntityRecord {
                name: world.names.get(entity).map(|n| n.0.clone()).unwrap_or_default(),
                transform: *world.transforms.get(entity).unwrap(),
                renderable: assets.get(&entity).cloned(),
                scripts: world.scripts.get(entity).map(|list| list.0.clone()).unwrap_or_default(),
                camera: world.cameras.get(entity).copied(),
            })
            .collect();

        SceneFile { entities }.save(&self.root.join(path))
    }

    /// Loads the scene file at `path` (project-relative).
    pub fn load_scene(&self, path: &Path) -> anyhow::Result<SceneFile> {
        SceneFile::load(&self.root.join(path))
    }

    /// Lists `.ron` scene files already in the project (relative to the project root), for the
    /// Assets panel's Scenes section. Same shape as [`Project::list_scripts`].
    pub fn list_scenes(&self) -> Vec<PathBuf> {
        let Ok(read_dir) = fs::read_dir(self.scenes_dir()) else { return Vec::new() };
        let mut paths: Vec<PathBuf> = read_dir
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| path.extension().and_then(|e| e.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("ron")))
            .filter_map(|path| path.strip_prefix(&self.root).ok().map(to_portable_path))
            .collect();
        paths.sort();
        paths
    }

    /// Creates a new empty `.ron` scene file in `scenes/`, for the Assets panel's "New Scene"
    /// button — the create-from-scratch counterpart to [`Project::create_script`]. Never
    /// overwrites: tries `Scene.ron`, then `Scene2.ron`, ... Returns its project-relative path.
    pub fn create_scene(&self) -> anyhow::Result<PathBuf> {
        let dir = self.scenes_dir();
        fs::create_dir_all(&dir)?;

        let mut candidate = dir.join("Scene.ron");
        let mut suffix = 2;
        while candidate.exists() {
            candidate = dir.join(format!("Scene{suffix}.ron"));
            suffix += 1;
        }

        SceneFile::default().save(&candidate)?;
        Ok(to_portable_path(candidate.strip_prefix(&self.root)?))
    }

    /// Marks `scene` (project-relative) as the project's start scene and persists the manifest
    /// immediately, so it survives without an explicit project-level Save step. Normalizes
    /// `scene`'s separators (see [`to_portable_path`]) in case the caller built it some way other
    /// than one of this struct's own path-returning methods.
    pub fn set_start_scene(&mut self, scene: PathBuf) -> anyhow::Result<()> {
        self.manifest.start_scene = to_portable_path(&scene);
        fs::write(self.manifest_path(), ron::ser::to_string_pretty(&self.manifest, Default::default())?)?;
        Ok(())
    }

    /// Lists assets of the given kind already in the project (relative to the project root),
    /// for the Assets panel and the inspector's attach-renderable picker to share. Filtered to
    /// each kind's own extension(s) — `models_dir()` also holds `.mtl` files and any textures an
    /// imported `.obj` depends on (see `import_obj_dependencies`), neither of which is itself an
    /// attachable model.
    pub fn list_assets(&self, kind: RenderableKind) -> Vec<PathBuf> {
        let (dir, extensions): (PathBuf, &[&str]) = match kind {
            RenderableKind::Sprite => (self.textures_dir(), &["png", "jpg", "jpeg"]),
            RenderableKind::Model => (self.models_dir(), &["obj"]),
        };

        let Ok(read_dir) = fs::read_dir(&dir) else { return Vec::new() };
        let mut paths: Vec<PathBuf> = read_dir
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| extensions.contains(&ext.to_lowercase().as_str()))
            })
            .filter_map(|path| path.strip_prefix(&self.root).ok().map(to_portable_path))
            .collect();
        paths.sort();
        paths
    }

    /// Lists `.rhai` script files already in the project (relative to the project root), for
    /// the Assets panel and the inspector's attach-script picker to share. Same shape as
    /// [`Project::list_assets`], just for the one script kind instead of a
    /// [`RenderableKind`]-style enum.
    pub fn list_scripts(&self) -> Vec<PathBuf> {
        let Ok(read_dir) = fs::read_dir(self.scripts_dir()) else { return Vec::new() };
        let mut paths: Vec<PathBuf> = read_dir
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("rhai")))
            .filter_map(|path| path.strip_prefix(&self.root).ok().map(to_portable_path))
            .collect();
        paths.sort();
        paths
    }

    /// Creates a new `.rhai` file in the project's `scripts/` folder, pre-filled with
    /// `on_start`/`on_update` boilerplate (see [`SCRIPT_BOILERPLATE`]), and returns its path
    /// relative to the project root — for the Assets panel's "New Script" button, the
    /// create-from-scratch counterpart to [`Project::import_asset`]. Never overwrites an
    /// existing file: tries `Script.rhai`, then `Script2.rhai`, `Script3.rhai`, ... until it
    /// finds a name nothing is using yet.
    pub fn create_script(&self) -> anyhow::Result<PathBuf> {
        let dir = self.scripts_dir();
        fs::create_dir_all(&dir)?;

        let mut candidate = dir.join("Script.rhai");
        let mut suffix = 2;
        while candidate.exists() {
            candidate = dir.join(format!("Script{suffix}.rhai"));
            suffix += 1;
        }

        fs::write(&candidate, SCRIPT_BOILERPLATE)?;
        Ok(to_portable_path(candidate.strip_prefix(&self.root)?))
    }

    /// Copies `src` into the project's `assets/` folder, sorted into `textures/`/`models/`/
    /// `scripts/` by extension (anything else falls back to a flat `assets/` folder). Importing
    /// an `.obj` also copies its referenced `.mtl` file(s) and the textures those reference, so
    /// the model still loads from its new location. Returns the new path, relative to the
    /// project root.
    pub fn import_asset(&self, src: &Path) -> anyhow::Result<PathBuf> {
        let extension = src.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
        let dest_dir = match extension.as_str() {
            "png" | "jpg" | "jpeg" => self.textures_dir(),
            "obj" | "mtl" => self.models_dir(),
            "rhai" => self.scripts_dir(),
            _ => self.assets_dir(),
        };
        fs::create_dir_all(&dest_dir)?;

        let file_name = src.file_name().ok_or_else(|| anyhow::anyhow!("import source has no file name"))?;
        let dest = dest_dir.join(file_name);
        fs::copy(src, &dest)?;

        if extension == "obj" {
            Self::import_obj_dependencies(src, &dest_dir)?;
        }

        Ok(to_portable_path(dest.strip_prefix(&self.root)?))
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

#[cfg(test)]
mod tests {
    use super::*;
    use libdqg::camera::Camera;
    use libdqg::glam;
    use libdqg::world::Transform;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("libdqg_project_tests").join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_new_project_defaults_to_scenes_main_ron_as_its_start_scene() {
        let project = Project::create(temp_dir("default_start_scene")).expect("project should be created");
        assert_eq!(project.manifest.start_scene, PathBuf::from(DEFAULT_SCENE_FILE));
        assert!(project.start_scene_path().is_file(), "the default start scene should exist on disk");
    }

    #[test]
    fn create_scene_never_overwrites_an_existing_one() {
        let project = Project::create(temp_dir("create_scene_no_overwrite")).expect("project should be created");

        let first = project.create_scene().expect("first create_scene should succeed");
        let second = project.create_scene().expect("second create_scene should succeed");

        assert_ne!(first, second, "a second New Scene click shouldn't overwrite the first");
        assert!(project.root.join(&first).is_file());
        assert!(project.root.join(&second).is_file());
    }

    #[test]
    fn list_scenes_includes_every_ron_file_in_the_scenes_dir() {
        let project = Project::create(temp_dir("list_scenes")).expect("project should be created");
        let extra = project.create_scene().expect("create_scene should succeed");

        let scenes = project.list_scenes();

        assert!(scenes.contains(&PathBuf::from(DEFAULT_SCENE_FILE)));
        assert!(scenes.contains(&extra));
    }

    #[test]
    fn set_start_scene_persists_across_reopening_the_project() {
        let root = temp_dir("set_start_scene_persists");
        let mut project = Project::create(root.clone()).expect("project should be created");
        let extra = project.create_scene().expect("create_scene should succeed");

        project.set_start_scene(extra.clone()).expect("set_start_scene should succeed");

        let reopened = Project::open(root).expect("project should reopen");
        assert_eq!(reopened.manifest.start_scene, extra);
    }

    /// Regression test: `ProjectManifest`'s `start_scene` field was added after this format
    /// shipped, so a `project.ron` saved before that must still open, falling back to the same
    /// default a brand new project gets rather than failing to deserialize.
    #[test]
    fn a_manifest_saved_before_start_scene_existed_still_opens() {
        let root = temp_dir("manifest_missing_start_scene");
        fs::write(root.join(MANIFEST_FILE), "(format_version: 1, name: \"Old Project\")").unwrap();

        let project = Project::open(root).expect("a manifest predating start_scene should still open");

        assert_eq!(project.manifest.start_scene, PathBuf::from(DEFAULT_SCENE_FILE));
    }

    #[test]
    fn save_scene_then_load_scene_round_trips_at_a_given_path() {
        let project = Project::create(temp_dir("save_load_round_trip")).expect("project should be created");
        let scene_path = project.create_scene().expect("create_scene should succeed");
        let mut world = World::new(Camera {
            position: glam::Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            aspect: 1.0,
            fov: 45.0,
            znear: 0.1,
            zfar: 100.0,
        });
        world.spawn_empty("Placeholder", Transform::default());

        project.save_scene(&scene_path, &world, &HashMap::new()).expect("save_scene should succeed");
        let loaded = project.load_scene(&scene_path).expect("load_scene should succeed");

        assert_eq!(loaded.entities.len(), 1);
        assert_eq!(loaded.entities[0].name, "Placeholder");
    }
}
