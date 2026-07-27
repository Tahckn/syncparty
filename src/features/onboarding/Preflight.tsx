import { useCallback, useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { useAppState } from "@/app/AppState";
import { useTranslate, type Translate } from "@/shared/i18n";
import { ipc } from "@/shared/ipc";
import { Badge, Button, Card, Dot } from "@/shared/ui";
import type { AppMode } from "@/shared/types/AppMode";
import type { DependencyId } from "@/shared/types/DependencyId";
import type { PreflightItem } from "@/shared/types/PreflightItem";
import type { PreflightReport } from "@/shared/types/PreflightReport";

/**
 * The setup checklist.
 *
 * Runs on every visit rather than caching a "setup done" flag, because the
 * things it checks for can be uninstalled between launches.
 */
export function Preflight({
  mode,
  onReady,
}: {
  mode: AppMode;
  onReady: () => void;
}) {
  const t = useTranslate();
  const { installs, reportFailure } = useAppState();

  const [report, setReport] = useState<PreflightReport | null>(null);
  const [checking, setChecking] = useState(true);
  const [installing, setInstalling] = useState<DependencyId | null>(null);

  const check = useCallback(async () => {
    setChecking(true);
    try {
      setReport(await ipc.runPreflight(mode));
    } catch (error) {
      reportFailure(error);
    } finally {
      setChecking(false);
    }
  }, [mode, reportFailure]);

  useEffect(() => {
    void check();
  }, [check]);

  async function install(id: DependencyId) {
    setInstalling(id);
    try {
      await ipc.installDependency(id);
    } catch (error) {
      reportFailure(error);
    } finally {
      setInstalling(null);
      // Re-check rather than assuming: the install may have half-worked.
      await check();
    }
  }

  const satisfied =
    report !== null && report.items.every((item) => item.status.state !== "missing");

  return (
    <div className="mx-auto max-w-2xl space-y-4 px-6 py-8">
      <header>
        <h1 className="text-xl font-semibold text-ink">{t("preflight.title")}</h1>
        <p className="mt-1 text-sm text-ink-muted">{t("preflight.subtitle")}</p>
      </header>

      <Card>
        {checking && !report ? (
          <p className="py-6 text-center text-sm text-ink-faint">
            {t("preflight.checking")}
          </p>
        ) : (
          <ul className="divide-y divide-line">
            {report?.items.map((item) => (
              <DependencyRow
                key={item.id}
                item={item}
                busy={installing === item.id}
                progress={installs[item.id]?.stage}
                disabled={installing !== null}
                onInstall={() => void install(item.id)}
              />
            ))}
          </ul>
        )}
      </Card>

      <div className="flex items-center justify-between gap-3">
        <Button
          variant="ghost"
          onClick={() => void check()}
          disabled={checking || installing !== null}
        >
          {checking ? t("preflight.checking") : t("preflight.recheck")}
        </Button>

        <div className="flex items-center gap-3">
          {satisfied && (
            <span className="text-sm text-good">{t("preflight.allReady")}</span>
          )}
          <Button variant="primary" onClick={onReady} disabled={!satisfied}>
            {t("preflight.continue")}
          </Button>
        </div>
      </div>
    </div>
  );
}

function DependencyRow({
  item,
  busy,
  progress,
  disabled,
  onInstall,
}: {
  item: PreflightItem;
  busy: boolean;
  progress: string | undefined;
  disabled: boolean;
  onInstall: () => void;
}) {
  const t = useTranslate();
  const installed = item.status.state === "installed";

  return (
    <li className="flex items-center gap-3 py-3 first:pt-0 last:pb-0">
      <Dot tone={installed ? "good" : "warn"} />

      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-ink">
          {item.displayName}
        </p>
        <p className="truncate text-xs text-ink-faint">{detailFor(item, busy, progress, t)}</p>
      </div>

      {installed ? (
        <Badge tone="good">{t("preflight.installed")}</Badge>
      ) : item.canAutoInstall ? (
        <Button variant="primary" onClick={onInstall} disabled={disabled}>
          {busy ? t("preflight.installing") : t("preflight.install")}
        </Button>
      ) : (
        <Button onClick={() => void openUrl(item.manualUrl)}>
          {t("preflight.manual")}
        </Button>
      )}
    </li>
  );
}

/**
 * The line under a dependency's name.
 *
 * For something already installed this is the version, and an empty string
 * when the tool would not report one — the badge beside it already says
 * "Ready", so repeating that word here would be noise.
 */
function detailFor(
  item: PreflightItem,
  busy: boolean,
  progress: string | undefined,
  t: Translate,
): string {
  if (busy) return progress ?? t("preflight.installing");

  if (item.status.state === "installed") return item.status.version ?? "";

  if (!item.canAutoInstall) return t("preflight.noAutoInstall");

  return item.needsElevation
    ? t("preflight.elevation")
    : t("preflight.missing");
}
