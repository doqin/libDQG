use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use libdqg::scene_file::GameManifest;

use crate::project::Project;

/// Packages `project` into a standalone folder at `output_dir`: the prebuilt `runtime` export
/// template (see [`find_template`]), renamed to the project's name, plus a `res/` folder holding
/// the project's assets, scripts, and scene — the exact layout `runtime`'s own `res_dir`/
/// `load_manifest`/`load_scene_file` expect. Pure file copying, no compilation — see the
/// export/compile implementation plan for why.
pub fn export_project(project: &Project, output_dir: &Path) -> anyhow::Result<()> {
    let template = find_template()?;
    package_into(&template, project, output_dir)
}

/// The actual file-copying steps, factored out of [`export_project`] so they can be exercised in
/// a test without depending on [`find_template`]'s `env::current_exe`-based lookup.
fn package_into(template: &Path, project: &Project, output_dir: &Path) -> anyhow::Result<()> {
    let res_dir = output_dir.join("res");
    // Wipe any previous export's `res/` first — `fs_extra`'s `overwrite` only overwrites files
    // that still exist in the source, so a stale asset/script removed or renamed in the project
    // since the last export would otherwise linger here and the exported game could still load
    // it.
    if res_dir.is_dir() {
        fs::remove_dir_all(&res_dir)?;
    }
    fs::create_dir_all(&res_dir)?;

    let exe_name = format!("{}{}", sanitize_file_name(&project.manifest.name), env::consts::EXE_SUFFIX);
    fs::copy(template, output_dir.join(&exe_name))?;

    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.overwrite = true;

    // Stored asset/script paths (e.g. `assets/textures/foo.png`) are project-root-relative, so
    // mirroring `assets/`'s and `scripts/`'s own layout under `res/` needs no path rewriting —
    // `RenderableAsset::load`/`ScriptRuntime` resolve them against `res_dir` exactly as the
    // editor resolves them against `project.root`.
    let assets_dir = project.root.join("assets");
    if assets_dir.is_dir() {
        fs_extra::dir::copy(&assets_dir, &res_dir, &copy_options)?;
    }
    let scripts_dir = project.root.join("scripts");
    if scripts_dir.is_dir() {
        fs_extra::dir::copy(&scripts_dir, &res_dir, &copy_options)?;
    }

    let scene_src = project.root.join("scenes/main.ron");
    if scene_src.is_file() {
        fs::copy(&scene_src, res_dir.join("scene.ron"))?;
    }

    let manifest = GameManifest { title: project.manifest.name.clone(), width: 1280, height: 720 };
    fs::write(res_dir.join("game.ron"), ron::ser::to_string_pretty(&manifest, Default::default())?)?;

    Ok(())
}

/// Locates the prebuilt `runtime` export template next to the running editor binary: first a
/// `templates/` subfolder (how a packaged editor release ships it), then flat next to the editor
/// exe (how a local `cargo build` — which puts every workspace binary in the same
/// `target/<profile>/` directory — makes it available with no extra packaging step in dev).
fn find_template() -> anyhow::Result<PathBuf> {
    let exe_name = format!("runtime{}", env::consts::EXE_SUFFIX);
    let editor_dir = env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("could not determine the editor's own directory"))?
        .to_path_buf();

    let candidates = [editor_dir.join("templates").join(&exe_name), editor_dir.join(&exe_name)];
    candidates.into_iter().find(|path| path.is_file()).ok_or_else(|| {
        anyhow::anyhow!(
            "export template not found (looked for templates/{exe_name} and {exe_name} next to the editor) — \
             build the `runtime` crate first (cargo build -p runtime)"
        )
    })
}

/// Strips characters Windows filenames disallow from a project name, for the exported exe's
/// file name.
fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() { "Game".to_string() } else { trimmed.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join("libdqg_export_tests").join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn package_into_mirrors_project_layout_under_res() {
        let project_root = temp_dir("project_export_layout");
        let project = Project::create(project_root).expect("project should be created");

        fs::create_dir_all(project.textures_dir()).unwrap();
        fs::write(project.textures_dir().join("hero.png"), b"fake png bytes").unwrap();
        fs::create_dir_all(project.scripts_dir()).unwrap();
        fs::write(project.scripts_dir().join("move.rhai"), "let on_update = |dt, input| {};").unwrap();

        let template = temp_dir("template_export_layout").join("runtime.exe");
        fs::write(&template, b"fake exe bytes").unwrap();

        let output_dir = temp_dir("output_export_layout");
        package_into(&template, &project, &output_dir).expect("packaging should succeed");

        let exe_name = format!("{}{}", sanitize_file_name(&project.manifest.name), env::consts::EXE_SUFFIX);
        assert!(output_dir.join(&exe_name).is_file(), "template should be copied and renamed to the project name");
        assert!(output_dir.join("res/assets/textures/hero.png").is_file());
        assert!(output_dir.join("res/scripts/move.rhai").is_file());
        assert!(output_dir.join("res/scene.ron").is_file(), "an empty project still has scenes/main.ron to mirror");

        let manifest_text = fs::read_to_string(output_dir.join("res/game.ron")).unwrap();
        let manifest: GameManifest = ron::from_str(&manifest_text).unwrap();
        assert_eq!(manifest.title, project.manifest.name);
        assert_eq!(manifest.width, 1280);
        assert_eq!(manifest.height, 720);
    }

    #[test]
    fn sanitize_file_name_strips_characters_windows_disallows() {
        assert_eq!(sanitize_file_name("My:Game?"), "My_Game_");
        assert_eq!(sanitize_file_name("  "), "Game");
    }

    /// Regression test: re-exporting to the same output folder after an asset was removed from
    /// the project must not leave that asset behind in `res/` — see [`package_into`]'s
    /// `remove_dir_all` doc comment for why `fs_extra`'s `overwrite` alone isn't enough.
    #[test]
    fn package_into_removes_stale_files_from_a_previous_export() {
        let project_root = temp_dir("project_export_repeat");
        let project = Project::create(project_root).expect("project should be created");

        fs::create_dir_all(project.textures_dir()).unwrap();
        fs::write(project.textures_dir().join("old.png"), b"fake png bytes").unwrap();

        let template = temp_dir("template_export_repeat").join("runtime.exe");
        fs::write(&template, b"fake exe bytes").unwrap();

        let output_dir = temp_dir("output_export_repeat");
        package_into(&template, &project, &output_dir).expect("first export should succeed");
        assert!(output_dir.join("res/assets/textures/old.png").is_file());

        fs::remove_file(project.textures_dir().join("old.png")).unwrap();
        package_into(&template, &project, &output_dir).expect("second export should succeed");

        assert!(
            !output_dir.join("res/assets/textures/old.png").exists(),
            "an asset removed from the project should not survive a re-export to the same folder"
        );
    }
}
