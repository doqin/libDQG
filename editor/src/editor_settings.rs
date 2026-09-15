use std::fs;
use std::path::PathBuf;

const EDITOR_SETTINGS_FILE: &str = "editor_settings.ron";

/// Editor-wide preferences that aren't tied to any one project, persisted next to the editor
/// executable — same pattern as [`crate::recent_projects::RecentProjects`], just a different
/// piece of state. Currently just the external program "Open Script" (see
/// `ui::draw_script_tile`) launches scripts in, picked once and remembered.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct EditorSettings {
    pub preferred_editor: Option<PathBuf>,
}

impl EditorSettings {
    fn file_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(EDITOR_SETTINGS_FILE)))
            .unwrap_or_else(|| PathBuf::from(EDITOR_SETTINGS_FILE))
    }

    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(Self::file_path()) else { return Self::default() };
        ron::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        fs::write(Self::file_path(), ron::ser::to_string_pretty(self, Default::default())?)?;
        Ok(())
    }
}
