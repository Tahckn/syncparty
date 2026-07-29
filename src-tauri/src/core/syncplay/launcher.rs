//! Starting the Syncplay desktop client already pointed at a party.
//!
//! This is the whole point of the guest half: instead of reading an address,
//! a password and a room name out of a chat message and retyping all three
//! into a dialog, the guest clicks once.

use std::path::PathBuf;
use std::time::Duration;

use tokio::net::TcpStream;

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

#[cfg(windows)]
const VLC_FALLBACKS: &[&str] = &[
    r"C:\Program Files\VideoLAN\VLC\vlc.exe",
    r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
];

#[cfg(target_os = "macos")]
const VLC_FALLBACKS: &[&str] = &["/Applications/VLC.app/Contents/MacOS/VLC"];

#[cfg(not(any(windows, target_os = "macos")))]
const VLC_FALLBACKS: &[&str] = &["/usr/bin/vlc", "/usr/local/bin/vlc"];

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

/// Locates a player Syncplay can drive, preferring mpv when both are present.
pub fn find_player(manual: Option<&str>) -> Option<PathBuf> {
    manual
        .and_then(|raw| process::resolve_manual(raw, "mpv"))
        .or_else(|| manual.and_then(|raw| process::resolve_manual(raw, "vlc")))
        .or_else(|| process::locate("mpv", MPV_FALLBACKS))
        .or_else(|| process::locate("vlc", VLC_FALLBACKS))
}

/// How long to give one candidate address before moving on. Tailnet peers
/// answer in milliseconds when they answer at all, so this only has to be long
/// enough to survive a slow first handshake.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// Works out which of an invite's addresses actually reaches the server.
///
/// Necessary because the host cannot know which address a given guest should
/// use: a masqueraded address only resolves inside the tailnet the node was
/// shared into, and the host's own machine is not in that tailnet either.
/// Trying each one is cheap and removes the guesswork.
pub async fn reachable_host(invite: &Invite) -> Result<String> {
    let candidates = invite.candidates();

    for host in &candidates {
        let address = format!("{host}:{}", invite.port);

        if let Ok(Ok(stream)) =
            tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(&address)).await
        {
            // Nothing is sent on it; the connection was only ever the question.
            drop(stream);
            return Ok(host.clone());
        }
    }

    Err(SyncPartyError::MonitorFailed(format!(
        "none of these addresses answered on port {}: {}",
        invite.port,
        candidates.join(", ")
    )))
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
        let player_override = settings.executable_override(MPV_KEY);

        Ok(Self {
            client: find_client(client_override.as_deref())
                .ok_or_else(|| SyncPartyError::DependencyMissing("Syncplay".to_owned()))?,
            player: find_player(player_override.as_deref()),
        })
    }

    /// Launches the client into the party described by `invite`.
    ///
    /// The address is resolved first, because handing Syncplay one that cannot
    /// be reached makes it sit there and then quit with nothing useful said.
    ///
    /// The client is detached deliberately — it outlives syncparty, so closing
    /// this window mid-film does not kill the film.
    pub async fn join(&self, invite: &Invite, nickname: &str) -> Result<()> {
        let host = reachable_host(invite).await?;

        let mut command = process::spawnable(&self.client);
        command
            .args(self.arguments(invite, &host, nickname))
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
    fn arguments(&self, invite: &Invite, host: &str, nickname: &str) -> Vec<String> {
        let mut arguments = vec![
            "--host".to_owned(),
            format!("{host}:{}", invite.port),
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
            alternate_hosts: Vec::new(),
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

    fn arguments_for(launcher: &ClientLauncher) -> Vec<String> {
        launcher.arguments(&sample_invite(), "movie-box.tail1a2b3.ts.net", "ahmet")
    }

    #[test]
    fn folds_the_port_into_the_host_argument() {
        let arguments = arguments_for(&launcher_without_player());

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
    fn uses_the_resolved_address_rather_than_the_invite_primary() {
        let arguments =
            launcher_without_player().arguments(&sample_invite(), "100.79.178.123", "ahmet");

        let host_index = arguments
            .iter()
            .position(|a| a == "--host")
            .expect("--host");
        assert_eq!(
            arguments[host_index + 1],
            "100.79.178.123:8999",
            "whichever address answered is the one Syncplay must be given"
        );
    }

    #[test]
    fn always_passes_no_store_so_a_party_does_not_overwrite_saved_settings() {
        assert!(arguments_for(&launcher_without_player()).contains(&"--no-store".to_owned()));
    }

    #[test]
    fn omits_the_player_path_when_mpv_is_not_installed() {
        assert!(!arguments_for(&launcher_without_player())
            .iter()
            .any(|a| a == "--player-path"));
    }

    #[test]
    fn passes_the_player_path_when_mpv_is_present() {
        let launcher = ClientLauncher {
            client: PathBuf::from("/tmp/Syncplay"),
            player: Some(PathBuf::from("/usr/local/bin/mpv")),
        };

        let arguments = arguments_for(&launcher);
        let index = arguments
            .iter()
            .position(|a| a == "--player-path")
            .expect("--player-path");
        assert_eq!(arguments[index + 1], "/usr/local/bin/mpv");
    }

    #[tokio::test]
    async fn picks_the_address_that_answers_and_skips_the_ones_that_do_not() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let port = listener.local_addr().expect("addr").port();

        let invite = Invite {
            // 192.0.2.0/24 is reserved for documentation and routes nowhere,
            // so this stands in for an address that cannot be reached.
            host: "192.0.2.1".to_owned(),
            alternate_hosts: vec!["127.0.0.1".to_owned()],
            port,
            ..sample_invite()
        };

        assert_eq!(
            reachable_host(&invite).await.expect("one address answers"),
            "127.0.0.1"
        );
    }

    #[tokio::test]
    async fn reports_every_address_it_tried_when_none_answer() {
        let invite = Invite {
            host: "192.0.2.1".to_owned(),
            alternate_hosts: vec!["192.0.2.2".to_owned()],
            // Nothing listens here, and the port is in the message.
            port: 8999,
            ..sample_invite()
        };

        let error = reachable_host(&invite)
            .await
            .expect_err("nothing should answer");

        let message = error.to_string();
        assert!(message.contains("192.0.2.1"), "{message}");
        assert!(message.contains("192.0.2.2"), "{message}");
        assert!(message.contains("8999"), "{message}");
    }
    #[test]
    fn finds_vlc_in_a_manually_selected_folder() {
        let directory =
            std::env::temp_dir().join(format!("syncparty-vlc-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        let vlc = directory.join(if cfg!(windows) { "vlc.exe" } else { "vlc" });
        std::fs::write(&vlc, b"").expect("vlc");

        assert_eq!(
            find_player(directory.to_str()),
            Some(vlc),
            "a VLC folder should satisfy the player requirement"
        );
    }
}
