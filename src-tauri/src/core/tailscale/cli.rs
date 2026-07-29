//! [`TailscaleClient`] backed by the `tailscale` command line tool.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;

use crate::core::error::{Result, SyncPartyError};
use crate::core::process;
use crate::core::tailscale::{locator, AuthFlow, PeerIdentity, TailnetStatus, TailscaleClient};

/// How long to wait for the daemon to come up after `tailscale up`.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub struct CliTailscaleClient {
    executable: PathBuf,
}

impl CliTailscaleClient {
    /// Fails when Tailscale is not installed; preflight should have caught
    /// that first, so reaching this error means the user removed it mid-run.
    pub fn discover() -> Result<Self> {
        locator::find()
            .map(|executable| Self { executable })
            .ok_or_else(|| SyncPartyError::DependencyMissing("Tailscale".to_owned()))
    }

    pub fn at(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    async fn status_json(&self) -> Result<StatusJson> {
        let output = process::capture(&self.executable, ["status", "--json"]).await?;
        serde_json::from_str(&output.stdout).map_err(|error| {
            SyncPartyError::Other(format!("could not read the Tailscale status: {error}"))
        })
    }
}

#[async_trait]
impl TailscaleClient for CliTailscaleClient {
    async fn status(&self) -> Result<TailnetStatus> {
        let status = self.status_json().await?;

        Ok(TailnetStatus {
            is_running: status.backend_state == "Running",
            ipv4: status.self_node.as_ref().and_then(NodeJson::ipv4_string),
            dns_name: status.self_node.as_ref().and_then(NodeJson::clean_dns_name),
            backend_state: status.backend_state,
        })
    }

    async fn ipv4(&self) -> Result<Option<Ipv4Addr>> {
        // `tailscale ip -4` is cheaper than a full status dump and is the only
        // thing that matters while waiting for the daemon.
        let Some(output) = process::try_capture(&self.executable, ["ip", "-4"]).await else {
            return Ok(None);
        };

        Ok(output
            .stdout
            .lines()
            .map(str::trim)
            .find_map(|line| line.parse::<Ipv4Addr>().ok()))
    }

    async fn bring_up(&self) -> Result<AuthFlow> {
        if let Some(address) = self.ipv4().await? {
            if self.status().await?.is_running {
                return Ok(AuthFlow::Ready(address));
            }
        }

        // `tailscale up` blocks until the node is authenticated, so it runs
        // detached while the address is polled alongside it. Its output goes
        // nowhere on purpose — nothing reads these pipes, and a full pipe
        // buffer would wedge the child.
        let mut child = process::spawnable(&self.executable)
            .arg("up")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|error| SyncPartyError::CommandFailed {
                command: "tailscale up".to_owned(),
                status: "could not start".to_owned(),
                stderr: error.to_string(),
            })?;

        let deadline = tokio::time::Instant::now() + CONNECT_TIMEOUT;

        while tokio::time::Instant::now() < deadline {
            tokio::time::sleep(POLL_INTERVAL).await;

            if let Some(address) = self.ipv4().await? {
                let _ = child.kill().await;
                return Ok(AuthFlow::Ready(address));
            }

            // Surfaced to the caller rather than opened here, so the browser
            // opens exactly once no matter how long the sign-in takes.
            if let Some(auth_url) = self.status_json().await.ok().and_then(|s| s.auth_url) {
                if !auth_url.is_empty() {
                    // The daemon keeps the authorization request after the
                    // CLI disconnects; leaving this waiter alive would leak a
                    // process for every attempted sign-in.
                    let _ = child.kill().await;
                    return Ok(AuthFlow::NeedsLogin { auth_url });
                }
            }
        }

        let _ = child.kill().await;
        Err(SyncPartyError::TailscaleDown)
    }

    async fn whois(&self, ip: IpAddr) -> Result<PeerIdentity> {
        let output =
            process::capture(&self.executable, ["whois", "--json", &ip.to_string()]).await?;
        let whois: WhoisJson = serde_json::from_str(&output.stdout)?;

        let profile = whois.user_profile.unwrap_or_default();
        let hostname = whois
            .node
            .as_ref()
            .and_then(|node| node.host_info.as_ref())
            .and_then(|info| info.hostname.clone());

        let display_name = profile
            .display_name
            .or_else(|| profile.login_name.clone())
            .or_else(|| hostname.clone())
            .unwrap_or_else(|| "Unknown".to_owned());

        Ok(PeerIdentity {
            display_name,
            login_name: profile.login_name,
            hostname,
        })
    }

    async fn shareable_address(&self) -> Result<Option<String>> {
        let status = self.status_json().await?;

        // A node shared into someone else's tailnet is reached on a
        // masqueraded address. Ask each sharee peer what it sees us as.
        for peer in status.peers.values() {
            if !peer.sharee_node.unwrap_or(false) {
                continue;
            }

            let Some(peer_ip) = peer.tailscale_ips.first() else {
                continue;
            };

            let Some(output) =
                process::try_capture(&self.executable, ["whois", "--json", peer_ip]).await
            else {
                continue;
            };

            let masqueraded = serde_json::from_str::<WhoisJson>(&output.stdout)
                .ok()
                .and_then(|whois| whois.node)
                .and_then(|node| node.masquerade_address);

            if let Some(address) = masqueraded.filter(|value| !value.is_empty()) {
                return Ok(Some(address));
            }
        }

        let self_node = status.self_node.as_ref();
        Ok(self_node
            .and_then(NodeJson::clean_dns_name)
            .or_else(|| self_node.and_then(NodeJson::ipv4_string)))
    }
}

// The daemon's JSON has many more fields than syncparty reads; every struct
// here deliberately ignores the rest.

#[derive(Debug, Deserialize)]
struct StatusJson {
    #[serde(rename = "BackendState", default)]
    backend_state: String,
    #[serde(rename = "AuthURL")]
    auth_url: Option<String>,
    #[serde(rename = "Self")]
    self_node: Option<NodeJson>,
    #[serde(rename = "Peer", default)]
    peers: std::collections::HashMap<String, NodeJson>,
}

#[derive(Debug, Deserialize)]
struct NodeJson {
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Vec<String>,
    #[serde(rename = "ShareeNode")]
    sharee_node: Option<bool>,
}

impl NodeJson {
    fn ipv4_string(&self) -> Option<String> {
        self.tailscale_ips
            .iter()
            .find(|ip| ip.parse::<Ipv4Addr>().is_ok())
            .cloned()
    }

    fn clean_dns_name(&self) -> Option<String> {
        self.dns_name
            .as_deref()
            .map(|name| name.trim_end_matches('.'))
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
    }
}

#[derive(Debug, Deserialize)]
struct WhoisJson {
    #[serde(rename = "Node")]
    node: Option<WhoisNodeJson>,
    #[serde(rename = "UserProfile")]
    user_profile: Option<UserProfileJson>,
}

#[derive(Debug, Deserialize)]
struct WhoisNodeJson {
    #[serde(rename = "Hostinfo")]
    host_info: Option<HostInfoJson>,
    #[serde(rename = "SelfNodeV4MasqAddrForThisPeer")]
    masquerade_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HostInfoJson {
    #[serde(rename = "Hostname")]
    hostname: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct UserProfileJson {
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "LoginName")]
    login_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STATUS: &str = r#"{
        "BackendState": "Running",
        "AuthURL": "",
        "Self": {
            "DNSName": "movie-box.tail1a2b3.ts.net.",
            "TailscaleIPs": ["100.101.102.103", "fd7a:115c:a1e0::1"]
        },
        "Peer": {
            "nodekey:abc": {
                "DNSName": "friend.tail9z8y.ts.net.",
                "TailscaleIPs": ["100.64.0.9"],
                "ShareeNode": true
            }
        }
    }"#;

    #[test]
    fn reads_the_node_address_and_dns_name() {
        let status: StatusJson = serde_json::from_str(SAMPLE_STATUS).expect("parse");
        let self_node = status.self_node.expect("self node");

        assert_eq!(
            self_node.ipv4_string().as_deref(),
            Some("100.101.102.103"),
            "should pick the v4 address, not the v6 one"
        );
        assert_eq!(
            self_node.clean_dns_name().as_deref(),
            Some("movie-box.tail1a2b3.ts.net"),
            "the trailing dot should be stripped"
        );
    }

    #[test]
    fn recognises_sharee_peers() {
        let status: StatusJson = serde_json::from_str(SAMPLE_STATUS).expect("parse");

        let sharees: Vec<_> = status
            .peers
            .values()
            .filter(|peer| peer.sharee_node.unwrap_or(false))
            .collect();

        assert_eq!(sharees.len(), 1);
        assert_eq!(sharees[0].tailscale_ips[0], "100.64.0.9");
    }

    #[test]
    fn tolerates_a_status_with_nothing_in_it() {
        let status: StatusJson = serde_json::from_str("{}").expect("parse");

        assert!(status.self_node.is_none());
        assert!(status.peers.is_empty());
        assert_eq!(status.backend_state, "");
    }

    #[test]
    fn whois_falls_back_through_display_name_login_then_hostname() {
        let whois: WhoisJson = serde_json::from_str(
            r#"{"Node":{"Hostinfo":{"Hostname":"ahmet-pc"}},"UserProfile":{"LoginName":"ahmet@example.com"}}"#,
        )
        .expect("parse");

        let profile = whois.user_profile.expect("profile");
        assert!(profile.display_name.is_none());
        assert_eq!(profile.login_name.as_deref(), Some("ahmet@example.com"));
    }
}
