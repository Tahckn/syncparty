import { useEffect, useState } from "react";

import { useAppState } from "@/app/AppState";
import { useTranslate } from "@/shared/i18n";
import { errorMessage, ipc } from "@/shared/ipc";
import { Badge, Button, Card, Field, Input } from "@/shared/ui";
import type { Invite } from "@/shared/types/Invite";

/**
 * The guest half: accept an invite, then open Syncplay pointed at it.
 *
 * An invite arriving by deep link skips the paste step entirely, which is the
 * path this whole feature exists to make possible.
 */
export function GuestScreen() {
  const t = useTranslate();
  const { pendingInvite, clearPendingInvite, reportFailure } = useAppState();

  const [text, setText] = useState("");
  const [invite, setInvite] = useState<Invite | null>(null);
  const [fromLink, setFromLink] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);
  const [joined, setJoined] = useState(false);

  // A link that arrived while the app was open takes over the screen.
  useEffect(() => {
    if (!pendingInvite) return;

    setInvite(pendingInvite);
    setFromLink(true);
    setJoined(false);
    setParseError(null);
    clearPendingInvite();
  }, [pendingInvite, clearPendingInvite]);

  async function decode() {
    setParseError(null);
    try {
      setInvite(await ipc.decodeInvite(text));
      setFromLink(false);
    } catch (error) {
      setParseError(errorMessage(error));
    }
  }

  async function join() {
    if (!invite) return;

    try {
      await ipc.joinParty(invite);
      setJoined(true);
    } catch (error) {
      reportFailure(error);
    }
  }

  function reset() {
    setInvite(null);
    setJoined(false);
    setText("");
    setFromLink(false);
  }

  return (
    <div className="mx-auto max-w-xl space-y-4 px-6 py-8">
      <header className="flex items-center gap-3">
        <h1 className="text-xl font-semibold text-ink">{t("guest.title")}</h1>
        {fromLink && <Badge tone="accent">{t("guest.received")}</Badge>}
      </header>

      {invite ? (
        <Card title={t("guest.invite.title")}>
          <div className="space-y-4">
            <div>
              <p className="text-lg font-semibold text-ink">{invite.room}</p>
              <p className="selectable font-mono text-xs text-ink-faint">
                {invite.host}:{invite.port}
              </p>
            </div>

            {joined ? (
              <p className="rounded-lg border border-good/40 bg-good/10 p-3 text-sm text-good">
                {t("guest.joined")}
              </p>
            ) : (
              <Button variant="primary" className="w-full" onClick={() => void join()}>
                {t("guest.join")}
              </Button>
            )}

            <Button variant="ghost" className="w-full" onClick={reset}>
              {t("guest.clear")}
            </Button>
          </div>
        </Card>
      ) : (
        <Card>
          <div className="space-y-4">
            <Field label={t("guest.paste.label")} hint={t("guest.paste.hint")}>
              <Input
                value={text}
                autoFocus
                placeholder={t("guest.paste.placeholder")}
                onChange={(event) => setText(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && text.trim()) void decode();
                }}
              />
            </Field>

            {parseError && <p className="text-sm text-bad">{parseError}</p>}

            <Button
              variant="primary"
              className="w-full"
              disabled={!text.trim()}
              onClick={() => void decode()}
            >
              {t("guest.decode")}
            </Button>
          </div>
        </Card>
      )}
    </div>
  );
}
