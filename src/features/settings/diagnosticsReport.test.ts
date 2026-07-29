import { describe, expect, it } from "vitest";

import type { DiagnosticsReport } from "@/shared/types/DiagnosticsReport";

import { safeToShare } from "./diagnosticsReport";

describe("safeToShare", () => {
  it("keeps useful health data without exposing addresses or local paths", () => {
    const report: DiagnosticsReport = {
      appVersion: "0.2.1",
      operatingSystem: "windows",
      dependencies: {
        mode: "host",
        items: [
          {
            id: "mpv",
            displayName: "mpv",
            status: {
              state: "installed",
              version: "0.40",
              path: "C:\\Users\\Taha\\private\\mpv.exe",
            },
            canAutoInstall: true,
            needsElevation: false,
            manualUrl: "https://example.com",
            supportsManualPath: true,
            overridePath: "C:\\Users\\Taha\\private",
          },
        ],
      },
      tailnet: {
        backendState: "Running",
        ipv4: "100.64.0.10",
        dnsName: "private-device.example.ts.net",
        isRunning: true,
      },
      tailnetError: null,
      session: { phase: "idle" },
    };

    const shared = JSON.stringify(safeToShare(report));

    expect(shared).toContain('"version":"0.40"');
    expect(shared).toContain('"hasAddress":true');
    expect(shared).not.toContain("100.64.0.10");
    expect(shared).not.toContain("private-device");
    expect(shared).not.toContain("Taha");
    expect(shared).not.toContain("example.com");
  });

  it("redacts diagnostic error details", () => {
    const report = {
      appVersion: "0.2.1",
      operatingSystem: "windows",
      dependencies: { mode: "guest", items: [] },
      tailnet: null,
      tailnetError: "failed at C:\\private\\tailscale.exe",
      session: { phase: "failed", message: "secret detail" },
    } satisfies DiagnosticsReport;

    const shared = safeToShare(report);

    expect(shared.tailnetError).toBe("unavailable");
    expect(shared.session).toEqual({ phase: "failed" });
    expect(JSON.stringify(shared)).not.toContain("private");
    expect(JSON.stringify(shared)).not.toContain("secret detail");
  });
});
