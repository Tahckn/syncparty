import { useTranslate } from "@/shared/i18n";
import { Badge, Card, Dot } from "@/shared/ui";
import type { RoomSnapshot } from "@/shared/types/RoomSnapshot";
import type { WatcherView } from "@/shared/types/WatcherView";

/**
 * Who is in the room, what they have open, and whether they are ready.
 *
 * Driven entirely by pushed snapshots — the panel never asks.
 */
export function RoomPanel({
  snapshot,
  monitorAttached,
}: {
  snapshot: RoomSnapshot | null;
  monitorAttached: boolean;
}) {
  const t = useTranslate();

  if (!monitorAttached) {
    return (
      <Card title={t("host.room.title")}>
        <p className="text-sm text-ink-faint">{t("host.room.monitorOff")}</p>
      </Card>
    );
  }

  if (!snapshot?.connected) {
    return (
      <Card title={t("host.room.title")}>
        <p className="flex items-center gap-2 text-sm text-ink-faint">
          <Dot tone="warn" />
          {t("host.room.disconnected")}
        </p>
      </Card>
    );
  }

  if (snapshot.rooms.length === 0) {
    return (
      <Card title={t("host.room.title")}>
        <p className="text-sm text-ink-faint">{t("host.room.empty")}</p>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {snapshot.rooms.map((room) => (
        <Card
          key={room.name}
          title={room.name}
          action={
            <Badge tone="neutral">
              {room.watchers.length}
            </Badge>
          }
        >
          {!room.everyoneOnTheSameFile && (
            <div className="mb-3 rounded-lg border border-warn/40 bg-warn/10 p-3">
              <p className="text-sm font-medium text-warn">
                {t("host.room.mismatch")}
              </p>
              <p className="mt-0.5 text-xs text-ink-muted">
                {t("host.room.mismatchDetail")}
              </p>
            </div>
          )}

          <ul className="divide-y divide-line">
            {room.watchers.map((watcher) => (
              <WatcherRow key={watcher.name} watcher={watcher} />
            ))}
          </ul>
        </Card>
      ))}
    </div>
  );
}

function WatcherRow({ watcher }: { watcher: WatcherView }) {
  const t = useTranslate();

  return (
    <li className="flex items-center gap-3 py-2.5 first:pt-0 last:pb-0">
      <Dot tone={watcher.isReady ? "good" : "neutral"} />

      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-ink">
          {watcher.name}
          {watcher.isController && (
            <span className="ml-2 text-xs font-normal text-accent">★</span>
          )}
        </p>
        <p className="truncate text-xs text-ink-faint">
          {watcher.file ? watcher.file.name : t("host.room.noFile")}
        </p>
      </div>

      {watcher.file?.durationSeconds != null && (
        <span className="shrink-0 font-mono text-xs text-ink-faint">
          {formatDuration(watcher.file.durationSeconds)}
        </span>
      )}

      <Badge tone={watcher.isReady ? "good" : "neutral"}>
        {watcher.isReady ? t("host.room.ready") : t("host.room.notReady")}
      </Badge>
    </li>
  );
}

/** Seconds to `h:mm:ss`, or `m:ss` for anything under an hour. */
function formatDuration(totalSeconds: number): string {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainder = seconds % 60;

  const padded = (value: number) => String(value).padStart(2, "0");

  return hours > 0
    ? `${hours}:${padded(minutes)}:${padded(remainder)}`
    : `${minutes}:${padded(remainder)}`;
}
