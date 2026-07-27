//! Starting and stopping the Syncplay server process.

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;
use ts_rs::TS;

use crate::core::error::{Result, SyncPartyError};
use crate::core::events::{AppEvent, EventBus};
use crate::core::paths::AppPaths;
use crate::core::process;

/// How long to wait for the server to start listening before giving up.
const READY_TIMEOUT: Duration = Duration::from_secs(15);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    /// Plaintext. Reaches the server through the environment, never `argv`.
    pub password: String,
    /// Stable across restarts, or every room operator password breaks.
    pub salt: String,
    /// The Tailscale address to bind to. Binding here rather than to
    /// `0.0.0.0` is what keeps the server off the local network entirely.
    pub bind_address: Ipv4Addr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ServerState {
    Stopped,
    Running { port: u16 },
}

/// Owns the Syncplay server process.
///
/// A trait so the Python-backed implementation below can eventually be
/// replaced by a native one without anything above noticing.
#[async_trait]
pub trait ServerController: Send + Sync {
    async fn start(&self, config: &ServerConfig) -> Result<()>;

    /// Stops the server and nothing else.
    ///
    /// Explicitly *not* Tailscale: the prototype ran `tailscale down` on
    /// shutdown, which cut every other thing the machine used the tailnet for.
    async fn stop(&self) -> Result<()>;

    async fn state(&self) -> ServerState;
}

/// Runs Syncplay out of the `uv`-managed virtual environment.
pub struct UvManagedServer {
    paths: AppPaths,
    bus: Arc<dyn EventBus>,
    running: Mutex<Option<RunningServer>>,
}

struct RunningServer {
    child: Child,
    port: u16,
}

impl UvManagedServer {
    pub fn new(paths: AppPaths, bus: Arc<dyn EventBus>) -> Self {
        Self {
            paths,
            bus,
            running: Mutex::new(None),
        }
    }

    /// Builds the server's argument list.
    ///
    /// Note what is *not* here: `--password` and `--salt`. Syncplay reads both
    /// from `SYNCPLAY_PASSWORD` and `SYNCPLAY_SALT`, so keeping them out of
    /// `argv` keeps them out of the process table, where any local program
    /// could otherwise read them.
    fn arguments(&self, config: &ServerConfig) -> Vec<String> {
        vec![
            "-u".to_owned(), // unbuffered, so log lines arrive as they happen
            self.paths
                .server_entrypoint()
                .to_string_lossy()
                .into_owned(),
            "--port".to_owned(),
            config.port.to_string(),
            "--isolate-rooms".to_owned(),
            "--ipv4-only".to_owned(),
            "--interface-ipv4".to_owned(),
            config.bind_address.to_string(),
        ]
    }

    /// Polls the listening socket until the server answers.
    ///
    /// Deliberately not a check for the welcome banner: that string is
    /// translated, so matching on it breaks the moment the machine is not in
    /// English. A successful connection means the same thing in every locale.
    async fn await_ready(&self, config: &ServerConfig) -> Result<()> {
        let address = format!("{}:{}", config.bind_address, config.port);
        let deadline = tokio::time::Instant::now() + READY_TIMEOUT;

        while tokio::time::Instant::now() < deadline {
            if tokio::net::TcpStream::connect(&address).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(READY_POLL_INTERVAL).await;
        }

        Err(SyncPartyError::ServerStartFailed(format!(
            "nothing was listening on {address} after {} seconds",
            READY_TIMEOUT.as_secs()
        )))
    }

    /// Forwards the child's output to the UI and the log file.
    fn pump_output(&self, child: &mut Child) {
        let log_path = self.paths.server_log();
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Some(stdout) = child.stdout.take() {
            spawn_reader(BufReader::new(stdout), Arc::clone(&self.bus), false);
        }

        if let Some(stderr) = child.stderr.take() {
            spawn_reader(BufReader::new(stderr), Arc::clone(&self.bus), true);
        }
    }
}

fn spawn_reader<R>(reader: BufReader<R>, bus: Arc<dyn EventBus>, is_error: bool)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            bus.publish(AppEvent::ServerLog { line, is_error });
        }
    });
}

#[async_trait]
impl ServerController for UvManagedServer {
    async fn start(&self, config: &ServerConfig) -> Result<()> {
        let mut running = self.running.lock().await;

        if running.is_some() {
            return Err(SyncPartyError::ServerAlreadyRunning);
        }

        let python = self.paths.server_python();
        if !python.is_file() {
            return Err(SyncPartyError::DependencyMissing(
                "Syncplay server runtime".to_owned(),
            ));
        }

        let mut child = process::spawnable(&python)
            .args(self.arguments(config))
            .current_dir(self.paths.syncplay_source_dir())
            .env("SYNCPLAY_PASSWORD", &config.password)
            .env("SYNCPLAY_SALT", &config.salt)
            // Syncplay prints non-ASCII in several languages; without this the
            // child dies on a UnicodeEncodeError when the console code page
            // cannot represent them.
            .env("PYTHONIOENCODING", "utf-8")
            .spawn()
            .map_err(|error| SyncPartyError::ServerStartFailed(error.to_string()))?;

        self.pump_output(&mut child);

        let port = config.port;
        *running = Some(RunningServer { child, port });
        drop(running);

        // On failure the child is cleaned up rather than left half-started.
        if let Err(error) = self.await_ready(config).await {
            let _ = self.stop().await;
            return Err(error);
        }

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;

        let Some(mut server) = running.take() else {
            return Ok(());
        };

        let _ = server.child.kill().await;
        let _ = server.child.wait().await;
        Ok(())
    }

    async fn state(&self) -> ServerState {
        match self.running.lock().await.as_ref() {
            Some(server) => ServerState::Running { port: server.port },
            None => ServerState::Stopped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::NullEventBus;

    fn server() -> UvManagedServer {
        UvManagedServer::new(
            AppPaths::rooted_at(std::env::temp_dir().join("syncparty-server-test")),
            Arc::new(NullEventBus),
        )
    }

    fn config() -> ServerConfig {
        ServerConfig {
            port: 8999,
            password: "swordfish".to_owned(),
            salt: "PEPPER".to_owned(),
            bind_address: Ipv4Addr::new(100, 101, 102, 103),
        }
    }

    #[test]
    fn the_password_and_salt_never_reach_the_command_line() {
        let arguments = server().arguments(&config());

        assert!(
            !arguments.iter().any(|a| a.contains("swordfish")),
            "the password must travel by environment variable"
        );
        assert!(!arguments.iter().any(|a| a.contains("PEPPER")));
        assert!(!arguments.iter().any(|a| a == "--password"));
        assert!(!arguments.iter().any(|a| a == "--salt"));
    }

    #[test]
    fn binds_only_to_the_tailscale_address() {
        let arguments = server().arguments(&config());

        let index = arguments
            .iter()
            .position(|a| a == "--interface-ipv4")
            .expect("--interface-ipv4");
        assert_eq!(arguments[index + 1], "100.101.102.103");
        assert!(arguments.contains(&"--ipv4-only".to_owned()));
    }

    #[test]
    fn isolates_rooms_and_runs_python_unbuffered() {
        let arguments = server().arguments(&config());

        assert!(arguments.contains(&"--isolate-rooms".to_owned()));
        assert_eq!(arguments[0], "-u");
    }

    #[tokio::test]
    async fn a_fresh_controller_reports_itself_stopped() {
        assert_eq!(server().state().await, ServerState::Stopped);
    }

    #[tokio::test]
    async fn stopping_something_that_never_started_is_not_an_error() {
        assert!(server().stop().await.is_ok());
    }
}
