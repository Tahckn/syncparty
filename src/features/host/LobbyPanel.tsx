import { useTranslate } from "@/shared/i18n";
import { Badge, Card, Dot } from "@/shared/ui";
import type { RoomSnapshot } from "@/shared/types/RoomSnapshot";

import { getLobbyState } from "./lobbyState";

export function LobbyPanel({
  snapshot,
}: {
  snapshot: RoomSnapshot | null;
}) {
  const t = useTranslate();

  const { people, readyCount, fileCount, filesCompatible, everyoneReady } =
    getLobbyState(snapshot);

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
    </Card>
  );
}
