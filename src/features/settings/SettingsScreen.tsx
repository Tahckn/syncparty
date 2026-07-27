import { useEffect, useState } from "react";

import { useAppState } from "@/app/AppState";
import { useTranslate } from "@/shared/i18n";
import { errorMessage, ipc } from "@/shared/ipc";
import { Button, Card, Field, Input, Toggle } from "@/shared/ui";
import type { AppMode } from "@/shared/types/AppMode";

export function SettingsScreen() {
  const t = useTranslate();
  const { settings, patchSettings, reportFailure } = useAppState();

  if (!settings) {
    return <p className="p-6 text-sm text-ink-faint">{t("common.loading")}</p>;
  }

  return (
    <div className="mx-auto max-w-2xl space-y-4 px-6 py-6">
      <h1 className="text-xl font-semibold text-ink">{t("settings.title")}</h1>

      <Card title={t("settings.general")}>
        <div className="space-y-4">
          <Field label={t("settings.nickname")} hint={t("settings.nickname.hint")}>
            <Input
              defaultValue={settings.nickname}
              onBlur={(event) => {
                const nickname = event.target.value.trim();
                if (nickname && nickname !== settings.nickname) {
                  void patchSettings({ nickname }).catch(reportFailure);
                }
              }}
            />
          </Field>

          <div className="grid gap-4 sm:grid-cols-2">
            <Field label={t("settings.room")}>
              <Input
                defaultValue={settings.room}
                onBlur={(event) => {
                  const room = event.target.value.trim();
                  if (room && room !== settings.room) {
                    void patchSettings({ room }).catch(reportFailure);
                  }
                }}
              />
            </Field>

            <Field label={t("settings.port")}>
              <Input
                type="number"
                min={1024}
                max={65535}
                defaultValue={settings.port}
                onBlur={(event) => {
                  const port = Number(event.target.value);
                  // Anything below 1024 needs elevation on both platforms.
                  if (port >= 1024 && port <= 65535 && port !== settings.port) {
                    void patchSettings({ port }).catch(reportFailure);
                  }
                }}
              />
            </Field>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <Choice
              label={t("settings.language")}
              value={settings.language}
              options={[
                { value: "en", label: "English" },
                { value: "tr", label: "Türkçe" },
              ]}
              onChange={(language) =>
                void patchSettings({ language }).catch(reportFailure)
              }
            />

            <Choice
              label={t("settings.mode")}
              value={settings.mode ?? "host"}
              options={[
                { value: "host", label: t("mode.host") },
                { value: "guest", label: t("mode.guest") },
              ]}
              onChange={(mode) =>
                void patchSettings({ mode: mode as AppMode }).catch(reportFailure)
              }
            />
          </div>
        </div>
      </Card>

      <Card title={t("settings.monitor")}>
        <Toggle
          checked={settings.monitorEnabled}
          label={t("settings.monitor")}
          hint={t("settings.monitor.hint")}
          onChange={(monitorEnabled) =>
            void patchSettings({ monitorEnabled }).catch(reportFailure)
          }
        />
      </Card>

      <DiscordSettings
        enabled={settings.discordEnabled}
        onToggle={(discordEnabled) =>
          void patchSettings({ discordEnabled }).catch(reportFailure)
        }
      />
    </div>
  );
}

function DiscordSettings({
  enabled,
  onToggle,
}: {
  enabled: boolean;
  onToggle: (next: boolean) => void;
}) {
  const t = useTranslate();

  const [configured, setConfigured] = useState(false);
  const [url, setUrl] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    void ipc.discordStatus().then(setConfigured);
  }, []);

  async function attempt(action: () => Promise<unknown>, success: string) {
    setNotice(null);
    setProblem(null);
    try {
      await action();
      setConfigured(await ipc.discordStatus());
      setNotice(success);
    } catch (error) {
      setProblem(errorMessage(error));
    }
  }

  return (
    <Card title={t("settings.discord")}>
      <div className="space-y-4">
        <Toggle
          checked={enabled}
          label={t("settings.discord.enable")}
          onChange={onToggle}
        />

        <Field label={t("settings.discord.webhook")}>
          <Input
            type="url"
            value={url}
            placeholder={t("settings.discord.webhook.placeholder")}
            onChange={(event) => setUrl(event.target.value)}
          />
        </Field>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="primary"
            disabled={!url.trim()}
            onClick={() =>
              void attempt(async () => {
                await ipc.setDiscordWebhook(url);
                setUrl("");
              }, t("settings.saved"))
            }
          >
            {t("common.save")}
          </Button>

          <Button
            disabled={!configured}
            onClick={() =>
              void attempt(ipc.testDiscordWebhook, t("settings.discord.sent"))
            }
          >
            {t("settings.discord.test")}
          </Button>

          <Button
            variant="ghost"
            disabled={!configured}
            onClick={() =>
              void attempt(ipc.clearDiscordWebhook, t("settings.saved"))
            }
          >
            {t("settings.discord.clear")}
          </Button>
        </div>

        <p className="text-xs text-ink-faint">
          {configured
            ? t("settings.discord.configured")
            : t("settings.discord.notConfigured")}
        </p>

        {notice && <p className="text-sm text-good">{notice}</p>}
        {problem && <p className="text-sm text-bad">{problem}</p>}
      </div>
    </Card>
  );
}

/** A small segmented control; there are never more than a few options. */
function Choice({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-1.5">
      <span className="text-sm font-medium text-ink">{label}</span>
      <div className="flex gap-1 rounded-lg border border-line bg-canvas p-1">
        {options.map((option) => (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            className={
              option.value === value
                ? "flex-1 rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-accent-ink"
                : "flex-1 rounded-md px-3 py-1.5 text-sm text-ink-muted hover:text-ink"
            }
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
