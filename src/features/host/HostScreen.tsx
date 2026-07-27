import { useState } from "react";

import { useAppState } from "@/app/AppState";
import { useTranslate, type MessageKey } from "@/shared/i18n";
import { ipc } from "@/shared/ipc";
import { Badge, Button, Card, Dot } from "@/shared/ui";
import type { StartupStep } from "@/shared/types/StartupStep";

import { InviteCard } from "./InviteCard";
import { RoomPanel } from "./RoomPanel";

const STEP_LABELS: Record<StartupStep, MessageKey> = {
  connectingTailscale: "host.step.connectingTailscale",
  startingServer: "host.step.startingServer",
  attachingMonitor: "host.step.attachingMonitor",
};

export function HostScreen() {
  const t = useTranslate();
  const { session, room, serverLog, reportFailure } = useAppState();

  const [busy, setBusy] = useState(false);
  const [logOpen, setLogOpen] = useState(false);

  const starting = session.phase === "starting";
  const hosting = session.phase === "hosting";

  async function run(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
    } catch (error) {
      reportFailure(error);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mx-auto max-w-3xl space-y-4 px-6 py-6">
      <Card>
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <Dot tone={hosting ? "good" : starting ? "warn" : "neutral"} />
              <h1 className="text-base font-semibold text-ink">
                {t("host.title")}
              </h1>
              {hosting && <Badge tone="good">{t("host.live")}</Badge>}
            </div>
            <p className="mt-1 text-sm text-ink-muted">
              {starting
                ? t(STEP_LABELS[session.step])
                : hosting
                  ? `${session.invite.host}:${session.invite.port}`
                  : t("host.idle.hint")}
            </p>
          </div>

          {hosting ? (
            <Button
              variant="danger"
              disabled={busy}
              onClick={() => void run(ipc.stopHosting)}
            >
              {t("host.stop")}
            </Button>
          ) : (
            <Button
              variant="primary"
              disabled={busy || starting}
              onClick={() => void run(ipc.startHosting)}
            >
              {busy || starting ? t("host.starting") : t("host.start")}
            </Button>
          )}
        </div>
      </Card>

      {hosting && (
        <>
          <InviteCard hosting={session} />
          <RoomPanel snapshot={room} monitorAttached={session.monitorAttached} />
        </>
      )}

      <Card
        title={t("host.logs.title")}
        action={
          <Button variant="ghost" onClick={() => setLogOpen((open) => !open)}>
            {logOpen ? t("host.logs.hide") : t("host.logs.show")}
          </Button>
        }
      >
        {logOpen ? (
          serverLog.length === 0 ? (
            <p className="text-sm text-ink-faint">{t("host.logs.empty")}</p>
          ) : (
            <pre className="selectable max-h-64 overflow-auto rounded-lg bg-canvas p-3 font-mono text-xs leading-relaxed text-ink-muted">
              {serverLog.join("\n")}
            </pre>
          )
        ) : (
          <p className="text-sm text-ink-faint">
            {serverLog.length} {t("host.logs.title").toLowerCase()}
          </p>
        )}
      </Card>
    </div>
  );
}
