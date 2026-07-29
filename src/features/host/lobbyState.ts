import type { RoomSnapshot } from "@/shared/types/RoomSnapshot";

const MONITOR_NAME = "syncparty-panel";

export function getLobbyState(snapshot: RoomSnapshot | null) {
  const people =
    snapshot?.rooms
      .flatMap((room) => room.watchers)
      .filter((watcher) => watcher.name !== MONITOR_NAME) ?? [];
  const readyCount = people.filter((person) => person.isReady).length;
  const fileCount = people.filter((person) => person.file != null).length;
  const filesCompatible =
    snapshot?.rooms.every(
      (room) =>
        room.fileCompatibility === "exact" ||
        room.fileCompatibility === "durationMatch",
    ) ?? false;

  return {
    people,
    readyCount,
    fileCount,
    filesCompatible,
    everyoneReady:
      people.length > 0 &&
      readyCount === people.length &&
      fileCount === people.length &&
      filesCompatible,
  };
}
