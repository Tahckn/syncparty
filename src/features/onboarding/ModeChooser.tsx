import { useTranslate } from "@/shared/i18n";
import type { AppMode } from "@/shared/types/AppMode";

export function ModeChooser({
  onChoose,
}: {
  onChoose: (mode: AppMode) => void;
}) {
  const t = useTranslate();

  return (
    <div className="mx-auto flex min-h-full max-w-4xl flex-col justify-center px-8 py-12">
      <div className="max-w-2xl">
        <div className="mb-5 flex items-center gap-3 text-xs font-bold tracking-[0.2em] text-accent uppercase">
          <span className="h-px w-8 bg-accent/70" />
          {t("onboarding.eyebrow")}
        </div>
        <h1 className="text-4xl font-bold tracking-[-0.04em] text-ink">
          {t("onboarding.title")}
        </h1>
        <p className="mt-3 max-w-xl text-base leading-relaxed text-ink-muted">{t("onboarding.subtitle")}</p>
      </div>

      <div className="mt-9 grid gap-4 sm:grid-cols-2">
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
      className="group relative overflow-hidden rounded-panel border border-line/70 bg-surface/75 p-6 text-left shadow-[0_18px_55px_oklch(0.08_0.03_275/0.22)] backdrop-blur-xl transition-all duration-300 hover:-translate-y-1 hover:border-accent/70 hover:bg-surface-raised/85"
    >
      <span
        aria-hidden
        className="flex size-11 items-center justify-center rounded-xl border border-accent/15 bg-accent/12 text-xl text-accent transition-transform duration-300 group-hover:scale-110"
      >
        {glyph}
      </span>
      <h2 className="mt-6 text-lg font-bold tracking-tight text-ink">{title}</h2>
      <p className="mt-2 text-sm leading-relaxed text-ink-muted">{detail}</p>
      <span aria-hidden className="absolute right-5 bottom-5 text-xl text-ink-faint transition-all group-hover:translate-x-1 group-hover:text-accent">→</span>
    </button>
  );
}
