//! Starting the Syncplay desktop client already pointed at a party.
//!
//! This is the whole point of the guest half: instead of reading an address,
//! a password and a room name out of a chat message and retyping all three
//! into a dialog, the guest clicks once.

use std::path::PathBuf;

use crate::core::config::ConfigStore;
use crate::core::error::{Result, SyncPartyError};
use crate::core::invite::Invite;
use crate::core::process;

#[cfg(windows)]
const CLIENT_FALLBACKS: &[&str] = &[
    r"C:\Program Files (x86)\Syncplay\Syncplay.exe",
    r"C:\Program Files\Syncplay\Syncplay.exe",
];

#[cfg(target_os = "macos")]
const CLIENT_FALLBACKS: &[&str] = &["/Applications/Syncplay.app/Contents/MacOS/Syncplay"];

#[cfg(not(any(windows, target_os = "macos")))]
const CLIENT_FALLBACKS: &[&str] = &["/usr/bin/syncplay", "/usr/local/bin/syncplay"];

#[cfg(windows)]
const MPV_FALLBACKS: &[&str] = &[
    r"C:\Program Files\mpv\mpv.exe",
    r"C:\Program Files\mpv.net\mpvnet.exe",
];

#[cfg(target_os = "macos")]
const MPV_FALLBACKS: &[&str] = &[
    "/Applications/mpv.app/Contents/MacOS/mpv",
    "/opt/homebrew/bin/mpv",
    "/usr/local/bin/mpv",
];

#[cfg(not(any(windows, target_os = "macos")))]
const MPV_FALLBACKS: &[&str] = &["/usr/bin/mpv", "/usr/local/bin/mpv"];

/// Settings keys under which a manually chosen path is stored.
pub const SYNCPLAY_CLIENT_KEY: &str = "syncplayClient";
pub const MPV_KEY: &str = "mpv";

/// Locates the Syncplay client executable.
///
/// A path the user set by hand wins over everything else — they told us where
/// it is, so second-guessing them would be strange.
pub fn find_client(manual: Option<&str>) -> Option<PathBuf> {
    manual
        .and_then(|raw| process::resolve_manual(raw, "syncplay"))
        .or_else(|| process::locate("syncplay", CLIENT_FALLBACKS))
}

/// Locates mpv, the player Syncplay drives.
pub fn find_mpv(manual: Option<&str>) -> Option<PathBuf> {
    manual
        .and_then(|raw| process::resolve_manual(raw, "mpv"))
        .or_else(|| process::locate("mpv", MPV_FALLBACKS))
}

pub struct ClientLauncher {
    client: PathBuf,
    player: Option<PathBuf>,
}

impl ClientLauncher {
    /// Resolves both programs, honouring whatever the user pointed at.
    ///
    /// Reads the same overrides the preflight check does, so a dependency
    /// reported as ready is one this can actually launch.
    pub fn discover(settings: &ConfigStore) -> Result<Self> {
        let client_override = settings.executable_override(SYNCPLAY_CLIENT_KEY);
        let mpv_override = settings.executable_override(MPV_KEY);

        Ok(Self {
            client: find_client(client_override.as_deref())
                .ok_or_else(|| SyncPartyError::DependencyMissing("Syncplay".to_owned()))?,
            player: find_mpv(mpv_override.as_deref()),
        })
    }

    /// Launches the client into the party described by `invite`.
    ///
    /// The client is detached deliberately — it outlives syncparty, so closing
    /// this window mid-film does not kill the film.
    pub fn join(&self, invite: &Invite, nickname: &str) -> Result<()> {
        let mut command = process::spawnable(&self.client);
        command
            .args(self.arguments(invite, nickname))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        command
            .spawn()
            .map_err(|error| SyncPartyError::CommandFailed {
                command: self.client.to_string_lossy().into_owned(),
                status: "could not start".to_owned(),
                stderr: error.to_string(),
            })?;

        Ok(())
    }

    /// Builds the client's argument list.
    ///
    /// `--host` carries the port too: the client splits on the last colon, so
    /// there is no separate `--port` flag. `--no-store` keeps a one-off party
    /// from overwriting whatever the guest normally connects to.
    fn arguments(&self, invite: &Invite, nickname: &str) -> Vec<String> {
        let mut arguments = vec![
            "--host".to_owned(),
            format!("{}:{}", invite.host, invite.port),
            "--name".to_owned(),
            nickname.to_owned(),
            "--room".to_owned(),
            invite.room.clone(),
            "--password".to_owned(),
            invite.password.clone(),
            "--no-store".to_owned(),
        ];

        if let Some(player) = &self.player {
            arguments.push("--player-path".to_owned());
            arguments.push(player.to_string_lossy().into_owned());
        }

        arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_invite() -> Invite {
        Invite {
            host: "movie-box.tail1a2b3.ts.net".to_owned(),
            port: 8999,
            password: "swordfish".to_owned(),
            room: "MovieNight".to_owned(),
        }
    }

    fn launcher_without_player() -> ClientLauncher {
        ClientLauncher {
            client: PathBuf::from("/tmp/Syncplay"),
            player: None,
        }
    }

    #[test]
    fn folds_the_port_into_the_host_argument() {
        let arguments = launcher_without_player().arguments(&sample_invite(), "ahmet");

        let host_index = arguments
            .iter()
            .position(|a| a == "--host")
            .expect("--host");
        assert_eq!(arguments[host_index + 1], "movie-box.tail1a2b3.ts.net:8999");
        assert!(
            !arguments.iter().any(|a| a == "--port"),
            "the client has no --port flag"
        );
    }

    #[test]
    fn always_passes_no_store_so_a_party_does_not_overwrite_saved_settings() {
        let arguments = launcher_without_player().arguments(&sample_invite(), "ahmet");

        assert!(arguments.contains(&"--no-store".to_owned()));
    }

    #[test]
    fn omits_the_player_path_when_mpv_is_not_installed() {
        let arguments = launcher_without_player().arguments(&sample_invite(), "ahmet");

        assert!(!arguments.iter().any(|a| a == "--player-path"));
    }

    #[test]
    fn passes_the_player_path_when_mpv_is_present() {
        let launcher = ClientLauncher {
            client: PathBuf::from("/tmp/Syncplay"),
            player: Some(PathBuf::from("/usr/local/bin/mpv")),
        };

        let arguments = launcher.arguments(&sample_invite(), "ahmet");
        let index = arguments
            .iter()
            .position(|a| a == "--player-path")
            .expect("--player-path");
        assert_eq!(arguments[index + 1], "/usr/local/bin/mpv");
    }
}
