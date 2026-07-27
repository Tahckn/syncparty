//! mpv as a managed dependency.

use std::sync::Arc;

use async_trait::async_trait;

use crate::core::config::ConfigStore;
use crate::core::deps::installer::{install_and_verify, PackageManagedInstall, PackageSpec};
use crate::core::deps::{Dependency, DependencyId, DependencyStatus, ModeRequirement};
use crate::core::error::Result;
use crate::core::events::ProgressSink;
use crate::core::process;
use crate::core::syncplay::{find_mpv, MPV_KEY};

const DISPLAY_NAME: &str = "mpv";
const MANUAL_URL: &str = "https://mpv.io/installation/";

pub struct MpvDependency {
    installer: PackageManagedInstall,
    settings: Arc<ConfigStore>,
}

impl MpvDependency {
    pub fn new(settings: Arc<ConfigStore>) -> Self {
        Self {
            installer: PackageManagedInstall {
                display_name: DISPLAY_NAME,
                spec: PackageSpec {
                    winget_id: Some("shinchiro.mpv"),
                    brew_cask: Some("mpv"),
                },
            },
            settings,
        }
    }
}

#[async_trait]
impl Dependency for MpvDependency {
    fn id(&self) -> DependencyId {
        DependencyId::Mpv
    }

    fn display_name(&self) -> &str {
        DISPLAY_NAME
    }

    /// Syncplay can drive other players, but mpv is the one syncparty sets up
    /// and the only one it can promise works.
    fn required_for(&self) -> ModeRequirement {
        ModeRequirement::Both
    }

    async fn detect(&self) -> DependencyStatus {
        let manual = self.settings.executable_override(MPV_KEY);

        let Some(path) = find_mpv(manual.as_deref()) else {
            return DependencyStatus::Missing;
        };

        DependencyStatus::Installed {
            version: process::probe_version(&path, &["--version"]).await,
            path: Some(path.to_string_lossy().into_owned()),
        }
    }

    async fn install(&self, progress: &dyn ProgressSink) -> Result<()> {
        install_and_verify(self, &self.installer, progress).await
    }

    fn manual_url(&self) -> &str {
        MANUAL_URL
    }

    fn needs_elevation(&self) -> bool {
        false
    }

    async fn can_auto_install(&self) -> bool {
        self.installer.is_supported()
    }

    /// mpv is very often a portable build sitting in a folder somewhere.
    fn supports_manual_path(&self) -> bool {
        true
    }

    fn manual_path_key(&self) -> Option<&'static str> {
        Some(MPV_KEY)
    }
}
