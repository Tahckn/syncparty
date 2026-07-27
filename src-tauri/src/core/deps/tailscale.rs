//! Tailscale as a managed dependency.

use async_trait::async_trait;

use crate::core::deps::installer::{install_and_verify, PackageManagedInstall, PackageSpec};
use crate::core::deps::{Dependency, DependencyId, DependencyStatus, ModeRequirement};
use crate::core::error::Result;
use crate::core::events::ProgressSink;
use crate::core::process;
use crate::core::tailscale::find_tailscale;

const DISPLAY_NAME: &str = "Tailscale";
const MANUAL_URL: &str = "https://tailscale.com/download";

pub struct TailscaleDependency {
    installer: PackageManagedInstall,
}

impl TailscaleDependency {
    pub fn new() -> Self {
        Self {
            installer: PackageManagedInstall {
                display_name: DISPLAY_NAME,
                spec: PackageSpec {
                    winget_id: Some("Tailscale.Tailscale"),
                    brew_cask: Some("tailscale-app"),
                },
            },
        }
    }
}

impl Default for TailscaleDependency {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Dependency for TailscaleDependency {
    fn id(&self) -> DependencyId {
        DependencyId::Tailscale
    }

    fn display_name(&self) -> &str {
        DISPLAY_NAME
    }

    /// Both halves need it — the tunnel is what everything else runs over.
    fn required_for(&self) -> ModeRequirement {
        ModeRequirement::Both
    }

    async fn detect(&self) -> DependencyStatus {
        let Some(path) = find_tailscale() else {
            return DependencyStatus::Missing;
        };

        DependencyStatus::Installed {
            version: process::probe_version(&path, &["version"]).await,
            path: Some(path.to_string_lossy().into_owned()),
        }
    }

    async fn install(&self, progress: &dyn ProgressSink) -> Result<()> {
        install_and_verify(self, &self.installer, progress).await
    }

    fn manual_url(&self) -> &str {
        MANUAL_URL
    }

    /// Tailscale installs a system service, so this always prompts.
    fn needs_elevation(&self) -> bool {
        true
    }

    async fn can_auto_install(&self) -> bool {
        self.installer.is_supported()
    }
}
