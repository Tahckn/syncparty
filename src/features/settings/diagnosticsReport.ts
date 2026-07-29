import type { DiagnosticsReport } from "@/shared/types/DiagnosticsReport";

/** Removes local paths and private tailnet addresses from copied reports. */
export function safeToShare(report: DiagnosticsReport) {
  return {
    appVersion: report.appVersion,
    operatingSystem: report.operatingSystem,
    mode: report.dependencies.mode,
    dependencies: report.dependencies.items.map((item) => ({
      id: item.id,
      status: item.status.state,
      version: item.status.state === "installed" ? item.status.version : null,
    })),
    tailnet: report.tailnet
      ? {
          backendState: report.tailnet.backendState,
          isRunning: report.tailnet.isRunning,
          hasAddress: report.tailnet.ipv4 != null,
          hasDnsName: report.tailnet.dnsName != null,
        }
      : null,
    tailnetError: report.tailnetError ? "unavailable" : null,
    session: { phase: report.session.phase },
  };
}
