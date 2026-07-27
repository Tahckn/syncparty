//! Talking to the local Tailscale daemon.
//!
//! Only ever asked for things that change slowly — the node's address, whether
//! the backend is up, who a peer is. That is deliberate: the PowerShell
//! prototype shelled out to `tailscale status --json` every two seconds to
//! drive its dashboard, when the answer is fixed for the whole evening. Live
//! data comes from [`crate::core::syncplay::RoomMonitor`] instead.

mod cli;
mod locator;

use std::net::{IpAddr, Ipv4Addr};

use async_trait::async_trait;
use serde::Serialize;
use ts_rs::TS;

use crate::core::error::Result;

pub use cli::CliTailscaleClient;
pub use locator::find as find_tailscale;

/// What the daemon reports about this node.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TailnetStatus {
    /// `Running`, `Stopped`, `NeedsLogin`, and so on, straight from the daemon.
    pub backend_state: String,
    pub ipv4: Option<String>,
    /// MagicDNS name with the trailing dot stripped.
    pub dns_name: Option<String>,
    pub is_running: bool,
}

/// Outcome of asking Tailscale to come up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthFlow {
    /// Already connected; carries the node's IPv4 address.
    Ready(Ipv4Addr),
    /// The user has to finish an interactive sign-in at this URL.
    NeedsLogin { auth_url: String },
}

/// Who is behind a Tailscale IP.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PeerIdentity {
    pub display_name: String,
    pub login_name: Option<String>,
    pub hostname: Option<String>,
}

/// The operations syncparty needs from Tailscale.
///
/// A trait rather than a concrete type because the CLI backend here is the
/// pragmatic choice, not the fast one: the daemon exposes a local HTTP API
/// that avoids spawning a process per call. Swapping that in later should not
/// touch anything above this boundary.
#[async_trait]
pub trait TailscaleClient: Send + Sync {
    async fn status(&self) -> Result<TailnetStatus>;

    async fn ipv4(&self) -> Result<Option<Ipv4Addr>>;

    /// Brings the tailnet up, returning either the address or a sign-in URL.
    ///
    /// Returns the URL to the caller instead of opening a browser, which is
    /// what keeps it from being opened more than once.
    async fn bring_up(&self) -> Result<AuthFlow>;

    async fn whois(&self, ip: IpAddr) -> Result<PeerIdentity>;

    /// The address to hand out to guests.
    ///
    /// When this machine has been shared into somebody else's tailnet, they
    /// reach it on a masqueraded address rather than its own IP. Falls back to
    /// the MagicDNS name, then the raw IPv4.
    async fn shareable_address(&self) -> Result<Option<String>>;
}
