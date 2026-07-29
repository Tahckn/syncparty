import { useEffect, useMemo, useState } from "react";

import { useTranslate } from "@/shared/i18n";
import { Badge, Button, Card, Dot } from "@/shared/ui";
import type { RoomSnapshot } from "@/shared/types/RoomSnapshot";

const MONITOR_NAME = "syncparty-panel";

export function LobbyPanel({
  snapshot,
  monitorAttached,
  opening,
  opened,
  onStart,
}: {
  snapshot: RoomSnapshot | null;
  monitorAttached: boolean;
  opening: boolean;
  opened: boolean;
  onStart: () => Promise<void>;
}) {
  const t = useTranslate();
  const [countdown, setCountdown] = useState<number | null>(null);

  const people = useMemo(
    () =>
      snapshot?.rooms.flatMap((room) => room.watchers).filter(
        (watcher) => watcher.name !== MONITOR_NAME,
      ) ?? [],
    [snapshot],
  );
  const readyCount = people.filter((person) => person.isReady).length;
  const fileCount = people.filter((person) => person.file != null).length;
  const filesCompatible =
    snapshot?.rooms.every(
      (room) =>
        room.fileCompatibility === "exact" ||
        room.fileCompatibility === "durationMatch",
    ) ?? false;
  const everyoneReady =
    people.length > 0 &&
    readyCount === people.length &&
    fileCount === people.length &&
    filesCompatible;

  useEffect(() => {
    if (countdown == null) return;

    if (countdown === 0) {
      setCountdown(null);
      void onStart();
      return;
    }

    const timer = window.setTimeout(
      () => setCountdown((value) => (value == null ? null : value - 1)),
      1_000,
    );
    return () => window.clearTimeout(timer);
  }, [countdown, onStart]);

  const canStart = monitorAttached && everyoneReady && !opening && !opened;

  return (
    <Card
      title={t("host.lobby.title")}
      action={
        <Badge tone={everyoneReady ? "good" : "neutral"}>
          <Dot tone={everyoneReady ? "good" : "neutral"} />
          {everyoneReady ? t("host.lobby.ready") : t("host.lobby.waiting")}
        </Badge>
      }
      className="border-accent/20"
    >
      <div className="grid gap-4 sm:grid-cols-[1fr_auto] sm:items-center">
        <div>
          <p className="text-sm font-semibold text-ink">
            {t("host.lobby.readyCount")}: {readyCount}/{people.length}
          </p>
          <p className="mt-1 text-xs leading-relaxed text-ink-muted">
            {people.length === 0
              ? t("host.lobby.empty")
              : fileCount < people.length
                ? t("host.lobby.filesWaiting")
                : !filesCompatible
                  ? t("host.lobby.filesMismatch")
                  : everyoneReady
                    ? t("host.lobby.everyoneReady")
                    : t("host.lobby.peopleWaiting")}
          </p>
        </div>

        <Button
          variant="primary"
          className="min-w-40"
          disabled={!canStart || countdown != null}
          onClick={() => setCountdown(3)}
        >
          {countdown != null
            ? t("host.lobby.countdown").replace("{count}", String(countdown))
            : opened
              ? t("host.lobby.opened")
              : opening
                ? t("host.joining")
                : t("host.lobby.start")}
        </Button>
      </div>

      <p className="mt-4 border-t border-line/60 pt-3 text-xs text-ink-faint">
        {t("host.lobby.hint")}
      </p>
      <span className="sr-only" aria-live="polite">
        {countdown != null
          ? t("host.lobby.countdown").replace("{count}", String(countdown))
          : ""}
      </span>
    </Card>
  );
}
