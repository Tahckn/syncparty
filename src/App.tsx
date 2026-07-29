import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { AppStateProvider, useAppState } from "@/app/AppState";
import { GuestScreen } from "@/features/guest/GuestScreen";
import { HostScreen } from "@/features/host/HostScreen";
import { ModeChooser } from "@/features/onboarding/ModeChooser";
import { Preflight } from "@/features/onboarding/Preflight";
import { SettingsScreen } from "@/features/settings/SettingsScreen";
import {
  TranslationProvider,
  useTranslate,
  type MessageKey,
} from "@/shared/i18n";
import { Badge, Button } from "@/shared/ui";
import type { AppMode } from "@/shared/types/AppMode";

export default function App() {
  return (
    <AppStateProvider>
      <Localised />
    </AppStateProvider>
  );
}

/** Sits inside the state provider so it can read the chosen language. */
function Localised() {
  const { settings } = useAppState();

  return (
    <TranslationProvider language={settings?.language ?? "en"}>
      <Shell />
    </TranslationProvider>
  );
}

function Shell() {
  const t = useTranslate();
  const { settings, patchSettings, pendingInvite, reportFailure } =
    useAppState();

  const [showSettings, setShowSettings] = useState(false);
  const [setupConfirmed, setSetupConfirmed] = useState(false);

  // An invite arriving by link means the user is a guest tonight, whatever
  // they picked last time.
  useEffect(() => {
    if (pendingInvite && settings && settings.mode !== "guest") {
      void patchSettings({ mode: "guest" }).catch(reportFailure);
    }
  }, [pendingInvite, settings, patchSettings, reportFailure]);

  const chooseMode = (mode: AppMode) => {
    setSetupConfirmed(false);
    void patchSettings({ mode }).catch(reportFailure);
  };

  return (
    <div className="flex h-full flex-col">
      <Header
        mode={settings?.mode ?? null}
        settingsOpen={showSettings}
        onToggleSettings={() => setShowSettings((open) => !open)}
      />

      {/* Above the loading state on purpose: if settings fail to load, the
          reason has to be visible rather than hidden behind a spinner that
          never resolves. */}
      <FailureBanner />

      <main className="min-h-0 flex-1 overflow-y-auto">
        {!settings ? (
          <p className="p-10 text-center text-sm text-ink-faint">
            {t("common.loading")}
          </p>
        ) : showSettings ? (
          <SettingsScreen />
        ) : settings.mode === null ? (
          <ModeChooser onChoose={chooseMode} />
        ) : !setupConfirmed ? (
          <Preflight
            mode={settings.mode}
            onReady={() => setSetupConfirmed(true)}
          />
        ) : settings.mode === "host" ? (
          <HostScreen />
        ) : (
          <GuestScreen />
        )}
      </main>
    </div>
  );
}

function Header({
  mode,
  settingsOpen,
  onToggleSettings,
}: {
  mode: AppMode | null;
  settingsOpen: boolean;
  onToggleSettings: () => void;
}) {
  const t = useTranslate();

  return (
    <header className="flex shrink-0 items-center justify-between border-b border-line px-5 py-3">
      <div className="flex items-center gap-2.5">
        <span
          aria-hidden
          className="grid size-6 place-items-center rounded-md bg-accent text-xs font-bold text-accent-ink"
        >
          s
        </span>
        <span className="text-sm font-semibold tracking-tight text-ink">
          {t("appName")}
        </span>
        {mode && (
          <Badge tone="neutral">
            {t(mode === "host" ? "mode.host" : "mode.guest")}
          </Badge>
        )}
      </div>

      <Button variant="ghost" onClick={onToggleSettings}>
        {settingsOpen ? t("common.close") : t("common.settings")}
      </Button>
    </header>
  );
}

/**
 * Failures that arrive outside a command call, plus the Tailscale sign-in
 * prompt — which is not really an error, but needs the same treatment: say
 * what happened and offer the one action that fixes it.
 */
function FailureBanner() {
  const t = useTranslate();
  const { failure, dismissFailure } = useAppState();

  if (!failure) return null;

  const knownKeys: Record<string, MessageKey> = {
    tailscale_login_required: "error.tailscale_login_required",
    dependency_missing: "error.dependency_missing",
    no_tailscale_route: "error.no_tailscale_route",
    party_not_running: "error.party_not_running",
    party_unreachable: "error.party_unreachable",
  };
  const headline = knownKeys[failure.kind];
  const authUrl = failure.authUrl;

  return (
    <div className="shrink-0 border-b border-warn/40 bg-warn/10 px-5 py-3">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="text-sm font-medium text-warn">
            {headline ? t(headline) : t("error.title")}
          </p>
          {!authUrl && (
            <p className="mt-0.5 text-xs break-words text-ink-muted">
              {failure.message}
            </p>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-2">
          {authUrl && (
            <Button variant="primary" onClick={() => void openUrl(authUrl)}>
              {t("error.openLogin")}
            </Button>
          )}
          <Button variant="ghost" onClick={dismissFailure}>
            {t("common.close")}
          </Button>
        </div>
      </div>
    </div>
  );
}
