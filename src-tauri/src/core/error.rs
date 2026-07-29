//! The single error type crossing every `core` boundary.
//!
//! Errors surface in the UI, so each variant carries a message written for a
//! person rather than a stack trace. [`SyncPartyError::kind`] gives the
//! frontend a stable discriminant to branch on without parsing prose.

use serde::{Serialize, Serializer};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SyncPartyError>;

#[derive(Debug, Error)]
pub enum SyncPartyError {
    #[error("{0} is not installed")]
    DependencyMissing(String),

    #[error("could not install {name}: {reason}")]
    InstallFailed { name: String, reason: String },

    #[error("no automatic installer is available for {name} on this platform")]
    InstallUnsupported { name: String },

    #[error("Tailscale is not running")]
    TailscaleDown,

    #[error("Tailscale sign-in required")]
    TailscaleLoginRequired { auth_url: String },

    #[error("this machine has no Tailscale IPv4 address yet")]
    TailscaleNoAddress,

    #[error("the Syncplay server is not running")]
    ServerNotRunning,

    #[error("the Syncplay server is already running")]
    ServerAlreadyRunning,

    #[error("the Syncplay server failed to start: {0}")]
    ServerStartFailed(String),

    #[error("the invite code is not valid: {0}")]
    InvalidInvite(String),

    #[error("could not reach the room monitor: {0}")]
    MonitorFailed(String),

    /// Every address in the invite failed a `tailscale ping`, not just a TCP
    /// connect — Tailscale itself has no route to the host from here. By far
    /// the most common cause is that this device was never shared into (or
    /// dropped out of) the host's tailnet, not a syncparty bug.
    #[error(
        "could not find the host on Tailscale from this device. Ask them to check that this \
         device still has access to their machine (Tailscale admin console → Share), and that \
         Tailscale here is signed in and connected."
    )]
    NoTailscaleRoute,

    /// The host answered a Tailscale ping, so the two machines can see each
    /// other, but nothing was listening on the party's port.
    #[error(
        "found the host on Tailscale, but nothing answered on port {port} — the movie night may \
         have already ended, or the server was not started"
    )]
    PartyNotRunning { port: u16 },

    /// Every candidate address failed, but the ping diagnostic could not
    /// tell why (Tailscale itself may be unreachable, or every candidate came
    /// back "no response" rather than a clear yes/no).
    #[error("could not reach the host at any of these addresses on port {port}: {addresses}")]
    PartyUnreachable { addresses: String, port: u16 },

    #[error("{command} exited with status {status}: {stderr}")]
    CommandFailed {
        command: String,
        status: String,
        stderr: String,
    },

    #[error("could not read or write settings: {0}")]
    Config(String),

    #[error("could not read or write a stored secret: {0}")]
    Secret(String),

    #[error("network request failed: {0}")]
    Network(String),

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Other(String),
}

impl SyncPartyError {
    /// Stable discriminant for the frontend. Never change an existing string;
    /// the UI branches on these to decide which recovery action to offer.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::DependencyMissing(_) => "dependency_missing",
            Self::InstallFailed { .. } => "install_failed",
            Self::InstallUnsupported { .. } => "install_unsupported",
            Self::TailscaleDown => "tailscale_down",
            Self::TailscaleLoginRequired { .. } => "tailscale_login_required",
            Self::TailscaleNoAddress => "tailscale_no_address",
            Self::ServerNotRunning => "server_not_running",
            Self::ServerAlreadyRunning => "server_already_running",
            Self::ServerStartFailed(_) => "server_start_failed",
            Self::InvalidInvite(_) => "invalid_invite",
            Self::MonitorFailed(_) => "monitor_failed",
            Self::NoTailscaleRoute => "no_tailscale_route",
            Self::PartyNotRunning { .. } => "party_not_running",
            Self::PartyUnreachable { .. } => "party_unreachable",
            Self::CommandFailed { .. } => "command_failed",
            Self::Config(_) => "config",
            Self::Secret(_) => "secret",
            Self::Network(_) => "network",
            Self::Io(_) => "io",
            Self::Other(_) => "other",
        }
    }
}

/// Tauri requires command errors to be `Serialize`. Emitting `{kind, message}`
/// keeps the wire shape stable even as variants gain fields.
impl Serialize for SyncPartyError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SyncPartyError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<std::io::Error> for SyncPartyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for SyncPartyError {
    fn from(value: serde_json::Error) -> Self {
        Self::Other(format!("malformed JSON: {value}"))
    }
}

impl From<reqwest::Error> for SyncPartyError {
    fn from(value: reqwest::Error) -> Self {
        Self::Network(value.to_string())
    }
}

impl From<keyring::Error> for SyncPartyError {
    fn from(value: keyring::Error) -> Self {
        Self::Secret(value.to_string())
    }
}
