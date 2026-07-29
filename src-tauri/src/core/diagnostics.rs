//! A single, read-only health snapshot for troubleshooting a movie night.

use serde::Serialize;
use ts_rs::TS;

use crate::core::config::AppMode;
use crate::core::deps::{DependencyManager, PreflightReport};
use crate::core::session::{PartySession, SessionState};
use crate::core::tailscale::{CliTailscaleClient, TailnetStatus, TailscaleClient};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub app_version: String,
    pub operating_system: String,
    pub dependencies: PreflightReport,
    pub tailnet: Option<TailnetStatus>,
    /// Kept separate from `tailnet`: a missing CLI and a stopped daemon both
    /// need an explanation, but neither should make the whole report fail.
    pub tailnet_error: Option<String>,
    pub session: SessionState,
}

/// Collects independent checks without changing machine or session state.
pub async fn collect(
    dependencies: &DependencyManager,
    session: &PartySession,
    mode: AppMode,
) -> DiagnosticsReport {
    let dependency_check = dependencies.preflight(mode);
    let session_check = session.state();
    let tailnet_check = async {
        let client = CliTailscaleClient::discover()?;
        client.status().await
    };

    let (dependencies, session, tailnet_result) =
        tokio::join!(dependency_check, session_check, tailnet_check);
    let (tailnet, tailnet_error) = match tailnet_result {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(error.to_string())),
    };

    DiagnosticsReport {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        operating_system: std::env::consts::OS.to_owned(),
        dependencies,
        tailnet,
        tailnet_error,
        session,
    }
}
