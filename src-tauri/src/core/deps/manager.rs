//! The registry that turns individual [`Dependency`] implementations into a
//! single preflight check.

use crate::core::config::AppMode;
use crate::core::deps::{
    Dependency, DependencyId, MpvDependency, PreflightItem, PreflightReport,
    ServerRuntimeDependency, SyncplayClientDependency, TailscaleDependency,
};
use crate::core::error::{Result, SyncPartyError};
use crate::core::events::{DependencyProgress, EventBus, ProgressSink};
use crate::core::paths::AppPaths;

pub struct DependencyManager {
    dependencies: Vec<Box<dyn Dependency>>,
}

impl DependencyManager {
    /// The set syncparty ships with.
    pub fn standard(paths: AppPaths) -> Self {
        Self::with(vec![
            Box::new(TailscaleDependency::new()),
            Box::new(SyncplayClientDependency::new()),
            Box::new(MpvDependency::new()),
            Box::new(ServerRuntimeDependency::new(paths)),
        ])
    }

    pub fn with(dependencies: Vec<Box<dyn Dependency>>) -> Self {
        Self { dependencies }
    }

    fn find(&self, id: DependencyId) -> Option<&dyn Dependency> {
        self.dependencies
            .iter()
            .map(AsRef::as_ref)
            .find(|dependency| dependency.id() == id)
    }

    /// Probes every dependency the mode needs.
    ///
    /// Detections run concurrently because each one spawns at least one
    /// process, and doing them in series is the difference between a preflight
    /// screen that appears instantly and one that visibly stalls.
    pub async fn preflight(&self, mode: AppMode) -> PreflightReport {
        let relevant = self
            .dependencies
            .iter()
            .filter(|dependency| dependency.required_for().applies_to(mode));

        let items = futures::future::join_all(relevant.map(|dependency| async move {
            PreflightItem {
                id: dependency.id(),
                display_name: dependency.display_name().to_owned(),
                status: dependency.detect().await,
                can_auto_install: dependency.can_auto_install().await,
                needs_elevation: dependency.needs_elevation(),
                manual_url: dependency.manual_url().to_owned(),
            }
        }))
        .await;

        PreflightReport { mode, items }
    }

    /// Installs one dependency, streaming progress onto the event bus.
    pub async fn install(&self, id: DependencyId, bus: &dyn EventBus) -> Result<()> {
        let progress = DependencyProgress::new(bus, id);
        self.install_with(id, &progress).await
    }

    pub async fn install_with(&self, id: DependencyId, progress: &dyn ProgressSink) -> Result<()> {
        let dependency = self
            .find(id)
            .ok_or_else(|| SyncPartyError::Other(format!("unknown dependency: {id:?}")))?;

        dependency.install(progress).await
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::core::deps::{DependencyStatus, ModeRequirement};
    use crate::core::events::test_support::RecordingEventBus;
    use crate::core::events::AppEvent;

    struct FakeDependency {
        id: DependencyId,
        requirement: ModeRequirement,
        status: DependencyStatus,
    }

    impl FakeDependency {
        fn installed(id: DependencyId, requirement: ModeRequirement) -> Self {
            Self {
                id,
                requirement,
                status: DependencyStatus::Installed {
                    version: None,
                    path: None,
                },
            }
        }

        fn missing(id: DependencyId, requirement: ModeRequirement) -> Self {
            Self {
                id,
                requirement,
                status: DependencyStatus::Missing,
            }
        }
    }

    #[async_trait]
    impl Dependency for FakeDependency {
        fn id(&self) -> DependencyId {
            self.id
        }

        fn display_name(&self) -> &str {
            "fake"
        }

        fn required_for(&self) -> ModeRequirement {
            self.requirement
        }

        async fn detect(&self) -> DependencyStatus {
            self.status.clone()
        }

        async fn install(&self, progress: &dyn ProgressSink) -> Result<()> {
            progress.report("installing", Some(50), None);
            Ok(())
        }

        fn manual_url(&self) -> &str {
            "https://example.com"
        }

        fn needs_elevation(&self) -> bool {
            false
        }

        async fn can_auto_install(&self) -> bool {
            true
        }
    }

    fn manager() -> DependencyManager {
        DependencyManager::with(vec![
            Box::new(FakeDependency::installed(
                DependencyId::Tailscale,
                ModeRequirement::Both,
            )),
            Box::new(FakeDependency::missing(
                DependencyId::ServerRuntime,
                ModeRequirement::HostOnly,
            )),
        ])
    }

    #[tokio::test]
    async fn a_guest_is_not_asked_for_the_server_runtime() {
        let report = manager().preflight(AppMode::Guest).await;

        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].id, DependencyId::Tailscale);
        assert!(report.is_satisfied());
    }

    #[tokio::test]
    async fn a_host_sees_the_missing_server_runtime() {
        let report = manager().preflight(AppMode::Host).await;

        assert_eq!(report.items.len(), 2);
        assert!(!report.is_satisfied());
        assert_eq!(
            report.missing().map(|item| item.id).collect::<Vec<_>>(),
            vec![DependencyId::ServerRuntime]
        );
    }

    #[tokio::test]
    async fn installing_publishes_progress_for_that_dependency() {
        let bus = RecordingEventBus::default();

        manager()
            .install(DependencyId::Tailscale, &bus)
            .await
            .expect("install");

        let events = bus.events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            AppEvent::InstallProgress {
                dependency: DependencyId::Tailscale,
                percent: Some(50),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn installing_an_unknown_dependency_is_an_error_not_a_panic() {
        let bus = RecordingEventBus::default();

        let error = manager()
            .install(DependencyId::Mpv, &bus)
            .await
            .expect_err("unknown dependency");

        assert_eq!(error.kind(), "other");
    }
}
