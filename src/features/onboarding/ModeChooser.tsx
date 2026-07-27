import { useTranslate } from "@/shared/i18n";
import type { AppMode } from "@/shared/types/AppMode";

export function ModeChooser({
  onChoose,
}: {
  onChoose: (mode: AppMode) => void;
}) {
  const t = useTranslate();

  return (
    <div className="mx-auto flex h-full max-w-2xl flex-col justify-center px-6 py-10">
      <h1 className="text-2xl font-semibold text-ink">
        {t("onboarding.title")}
      </h1>
      <p className="mt-2 text-sm text-ink-muted">{t("onboarding.subtitle")}</p>

      <div className="mt-8 grid gap-3 sm:grid-cols-2">
        <ModeCard
          title={t("onboarding.host.title")}
          detail={t("onboarding.host.detail")}
          glyph="◉"
          onClick={() => onChoose("host")}
        />
        <ModeCard
          title={t("onboarding.guest.title")}
          detail={t("onboarding.guest.detail")}
          glyph="→"
          onClick={() => onChoose("guest")}
        />
      </div>
    </div>
  );
}

function ModeCard({
  title,
  detail,
  glyph,
  onClick,
}: {
  title: string;
  detail: string;
  glyph: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-panel border border-line bg-surface p-5 text-left transition-colors hover:border-accent hover:bg-surface-raised"
    >
      <span
        aria-hidden
        className="flex size-9 items-center justify-center rounded-lg bg-accent/15 text-lg text-accent"
      >
        {glyph}
      </span>
      <h2 className="mt-4 text-base font-semibold text-ink">{title}</h2>
      <p className="mt-1.5 text-sm text-ink-muted">{detail}</p>
    </button>
  );
}
