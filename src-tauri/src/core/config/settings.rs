//! User-visible settings and their on-disk representation.
//!
//! Nothing secret lives here — passwords, salts and webhook URLs go through
//! [`crate::core::config::SecretStore`] into the OS keychain instead.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::error::{Result, SyncPartyError};
use crate::core::paths::AppPaths;

/// Which half of the app the user is running. Chosen during onboarding and
/// switchable later; it decides which dependencies are required and which
/// screens are reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum AppMode {
    /// Runs the Syncplay server and hands out invites.
    Host,
    /// Joins somebody else's party.
    Guest,
}

pub const DEFAULT_PORT: u16 = 8999;
pub const DEFAULT_ROOM: &str = "MovieNight";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// `None` until onboarding completes.
    pub mode: Option<AppMode>,
    pub port: u16,
    pub room: String,
    pub nickname: String,
    /// BCP-47 tag; the UI ships `tr` and `en`.
    pub language: String,
    /// Whether the host attaches a hidden client to read live room state.
    /// Disabling it trades the rich panel for one fewer name in the user list.
    pub monitor_enabled: bool,
    pub discord_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mode: None,
            port: DEFAULT_PORT,
            room: DEFAULT_ROOM.to_owned(),
            nickname: default_nickname(),
            language: "en".to_owned(),
            monitor_enabled: true,
            discord_enabled: false,
        }
    }
}

fn default_nickname() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "guest".to_owned())
}

/// Reads and writes [`AppSettings`] as JSON, keeping an in-memory copy so the
/// hot paths never touch the disk.
#[derive(Debug)]
pub struct ConfigStore {
    paths: AppPaths,
    cached: Mutex<AppSettings>,
}

impl ConfigStore {
    /// Loads settings from disk. A missing file yields defaults; a corrupt one
    /// is reported rather than silently discarded, so the user can recover it.
    pub fn load(paths: AppPaths) -> Result<Self> {
        let file = paths.settings_file();

        let settings = match std::fs::read_to_string(&file) {
            Ok(raw) => serde_json::from_str(&raw).map_err(|error| {
                SyncPartyError::Config(format!("{} could not be parsed: {error}", file.display()))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
            Err(error) => return Err(error.into()),
        };

        Ok(Self {
            paths,
            cached: Mutex::new(settings),
        })
    }

    pub fn get(&self) -> AppSettings {
        self.cached.lock().expect("settings mutex poisoned").clone()
    }

    /// Applies `mutate` to the settings and persists the result.
    pub fn update(&self, mutate: impl FnOnce(&mut AppSettings)) -> Result<AppSettings> {
        let updated = {
            let mut guard = self.cached.lock().expect("settings mutex poisoned");
            mutate(&mut guard);
            guard.clone()
        };

        self.persist(&updated)?;
        Ok(updated)
    }

    /// Writes to a sibling temporary file and renames over the target, so an
    /// interrupted write can never leave truncated JSON behind.
    fn persist(&self, settings: &AppSettings) -> Result<()> {
        let target = self.paths.settings_file();
        let staging = target.with_extension("json.tmp");

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&staging, serde_json::to_vec_pretty(settings)?)?;
        std::fs::rename(&staging, &target)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_paths(label: &str) -> AppPaths {
        let dir = std::env::temp_dir().join(format!("syncparty-settings-{label}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        AppPaths::rooted_at(dir)
    }

    #[test]
    fn missing_file_yields_defaults() {
        let store = ConfigStore::load(temp_paths("missing")).expect("load");

        assert_eq!(store.get().port, DEFAULT_PORT);
        assert_eq!(store.get().mode, None);
    }

    #[test]
    fn updates_survive_a_reload() {
        let paths = temp_paths("roundtrip");

        let store = ConfigStore::load(paths.clone()).expect("load");
        store
            .update(|settings| {
                settings.mode = Some(AppMode::Host);
                settings.port = 9100;
            })
            .expect("update");

        let reloaded = ConfigStore::load(paths).expect("reload");
        assert_eq!(reloaded.get().mode, Some(AppMode::Host));
        assert_eq!(reloaded.get().port, 9100);
    }

    #[test]
    fn corrupt_file_is_reported_not_swallowed() {
        let paths = temp_paths("corrupt");
        std::fs::write(paths.settings_file(), b"{ not json").expect("seed");

        let error = ConfigStore::load(paths).expect_err("should refuse to load");
        assert_eq!(error.kind(), "config");
    }
}
