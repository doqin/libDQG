use std::fs;
use std::path::PathBuf;

use crate::project::MANIFEST_FILE;

const RECENT_PROJECTS_FILE: &str = "recent_projects.ron";
const MAX_ENTRIES: usize = 10;

/// The menu scene's "recently opened" list: project root folders, most recent first,
/// persisted next to the editor executable so it survives between runs.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct RecentProjects {
    pub roots: Vec<PathBuf>,
}

impl RecentProjects {
    fn file_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join(RECENT_PROJECTS_FILE)))
            .unwrap_or_else(|| PathBuf::from(RECENT_PROJECTS_FILE))
    }

    /// Loads the persisted list, dropping any entries whose project no longer exists on disk
    /// (moved, deleted, on an unmounted drive, ...) so the menu never offers a dead shortcut.
    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(Self::file_path()) else { return Self::default() };
        let mut recent: Self = ron::from_str(&text).unwrap_or_default();
        recent.roots.retain(|root| root.join(MANIFEST_FILE).is_file());
        recent
    }

    pub fn save(&self) -> anyhow::Result<()> {
        fs::write(Self::file_path(), ron::ser::to_string_pretty(self, Default::default())?)?;
        Ok(())
    }

    /// Moves `root` to the front of the list (adding it if new), capped at [`MAX_ENTRIES`].
    pub fn add(&mut self, root: PathBuf) {
        self.roots.retain(|existing| existing != &root);
        self.roots.insert(0, root);
        self.roots.truncate(MAX_ENTRIES);
    }
}
