use super::model::ThemePreference;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct StoredSettings {
    #[serde(default)]
    pub theme_preference: ThemePreference,
}

impl Default for StoredSettings {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::System,
        }
    }
}

pub(crate) fn load_settings() -> Result<StoredSettings> {
    let path = settings_path()?;
    load_settings_from_path(&path)
}

pub(crate) fn save_settings(settings: &StoredSettings) -> Result<()> {
    let path = settings_path()?;
    save_settings_to_path(&path, settings)
}

pub(crate) fn load_settings_from_path(path: &Path) -> Result<StoredSettings> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)
            .with_context(|| format!("parse settings from {}", path.display()))?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(StoredSettings::default()),
        Err(err) => Err(err).with_context(|| format!("read {}", path.display())),
    }
}

pub(crate) fn save_settings_to_path(path: &Path, settings: &StoredSettings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create settings directory {}", parent.display()))?;
    }

    let bytes = serde_json::to_vec_pretty(settings).context("serialize GUI settings")?;
    std::fs::write(path, bytes).with_context(|| format!("write {}", path.display()))
}

fn settings_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "YunDrone", "ble-command-gateway")
        .context("resolve GUI config directory")?;
    Ok(project_dirs.config_dir().join("settings.json"))
}
