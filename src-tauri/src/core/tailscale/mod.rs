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

/// What happened when pinging a target at the Tailscale layer.
///
/// This is a diagnostic, not a connectivity check for the app itself — it
/// exists to tell apart the two reasons a party is unreachable that look
/// identical from a plain TCP timeout: the joining device has no route to the
/// host at all (usually because it was never shared into the host's tailnet,
/// or the share lapsed), versus the two machines can see each other on
/// Tailscale just fine and the problem is with the server itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PingOutcome {
    /// The peer answered. Tailscale-layer connectivity is fine.
    Answered,
    /// Tailscale does not know this peer at all — `tailscale ping` returns
    /// this instantly, before attempting anything. The device has no route to
    /// the target: it was never shared with, or the share expired.
    UnknownPeer,
    /// The peer is known but did not answer in time. Could mean it is
    /// offline, its Tailscale is not signed in, or the network is just slow.
    NoResponse,
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

    /// Pings `target` at the Tailscale layer — not a connection to anything
    /// syncparty runs, just "can this machine and that one see each other on
    /// the tailnet at all". See [`PingOutcome`] for why this is worth asking.
    async fn ping(&self, target: &str) -> Result<PingOutcome>;

    /// The address to hand out to guests.
    ///
    /// When this machine has been shared into somebody else's tailnet, they
    /// reach it on a masqueraded address rather than its own IP. Falls back to
    /// the MagicDNS name, then the raw IPv4.
    async fn shareable_address(&self) -> Result<Option<String>>;
}
