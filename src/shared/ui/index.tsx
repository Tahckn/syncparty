/**
 * The handful of primitives this app needs.
 *
 * Hand-rolled rather than pulled from a component library: there are six of
 * them, they have no behaviour worth abstracting, and a registry plus its
 * dependency tree would outweigh the whole frontend.
 */
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";

export function cx(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}

// ------------------------------------------------------------------ Button

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";

const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary:
    "bg-accent text-accent-ink hover:bg-accent-strong disabled:hover:bg-accent",
  secondary:
    "bg-surface-raised text-ink border border-line hover:border-ink-faint",
  ghost: "text-ink-muted hover:text-ink hover:bg-surface-raised",
  danger: "bg-surface-raised text-bad border border-line hover:border-bad",
};

export function Button({
  variant = "secondary",
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  return (
    <button
      {...props}
      className={cx(
        "inline-flex items-center justify-center gap-2 rounded-lg px-3.5 py-2",
        "text-sm font-medium transition-colors",
        "disabled:cursor-not-allowed disabled:opacity-45",
        BUTTON_VARIANTS[variant],
        className,
      )}
    />
  );
}

// -------------------------------------------------------------------- Card

export function Card({
  title,
  action,
  className,
  children,
}: {
  title?: ReactNode;
  action?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section
      className={cx(
        "rounded-panel border border-line bg-surface",
        className,
      )}
    >
      {(title || action) && (
        <header className="flex items-center justify-between gap-3 border-b border-line px-4 py-3">
          <h2 className="text-sm font-semibold tracking-wide text-ink-muted uppercase">
            {title}
          </h2>
          {action}
        </header>
      )}
      <div className="p-4">{children}</div>
    </section>
  );
}

// ------------------------------------------------------------------- Badge

type BadgeTone = "neutral" | "good" | "warn" | "bad" | "accent";

const BADGE_TONES: Record<BadgeTone, string> = {
  neutral: "bg-surface-raised text-ink-muted",
  good: "bg-good/15 text-good",
  warn: "bg-warn/15 text-warn",
  bad: "bg-bad/15 text-bad",
  accent: "bg-accent/15 text-accent",
};

export function Badge({
  tone = "neutral",
  children,
}: {
  tone?: BadgeTone;
  children: ReactNode;
}) {
  return (
    <span
      className={cx(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1",
        "text-xs font-medium whitespace-nowrap",
        BADGE_TONES[tone],
      )}
    >
      {children}
    </span>
  );
}

/** A small filled circle, for status that reads faster than a word. */
export function Dot({ tone }: { tone: BadgeTone }) {
  const colours: Record<BadgeTone, string> = {
    neutral: "bg-ink-faint",
    good: "bg-good",
    warn: "bg-warn",
    bad: "bg-bad",
    accent: "bg-accent",
  };

  return <span className={cx("size-2 rounded-full", colours[tone])} />;
}

// ------------------------------------------------------------------- Input

export function Input({
  className,
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={cx(
        "w-full rounded-lg border border-line bg-canvas px-3 py-2",
        "text-sm text-ink placeholder:text-ink-faint",
        "cursor-text select-text",
        "focus:border-accent focus:outline-none",
        className,
      )}
    />
  );
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="block space-y-1.5">
      <span className="text-sm font-medium text-ink">{label}</span>
      {children}
      {hint && <span className="block text-xs text-ink-faint">{hint}</span>}
    </label>
  );
}

// ------------------------------------------------------------------ Toggle

export function Toggle({
  checked,
  onChange,
  label,
  hint,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
  hint?: string;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm font-medium text-ink">{label}</p>
        {hint && <p className="mt-1 text-xs text-ink-faint">{hint}</p>}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        onClick={() => onChange(!checked)}
        className={cx(
          "relative mt-0.5 h-6 w-11 shrink-0 rounded-full transition-colors",
          checked ? "bg-accent" : "bg-surface-raised border border-line",
        )}
      >
        <span
          className={cx(
            "absolute top-1 size-4 rounded-full transition-all",
            checked ? "left-6 bg-accent-ink" : "left-1 bg-ink-faint",
          )}
        />
      </button>
    </div>
  );
}

// ------------------------------------------------------------- Copyable row

/** A label, a monospace value, and a copy button. */
export function CopyRow({
  label,
  value,
  copyLabel,
  copiedLabel,
  onCopy,
  copied,
}: {
  label: string;
  value: string;
  copyLabel: string;
  copiedLabel: string;
  onCopy: () => void;
  copied: boolean;
}) {
  return (
    <div className="space-y-1.5">
      <p className="text-xs font-medium tracking-wide text-ink-faint uppercase">
        {label}
      </p>
      <div className="flex items-center gap-2">
        <code className="selectable min-w-0 flex-1 truncate rounded-lg border border-line bg-canvas px-3 py-2 font-mono text-xs text-ink">
          {value}
        </code>
        <Button onClick={onCopy} className="shrink-0">
          {copied ? copiedLabel : copyLabel}
        </Button>
      </div>
    </div>
  );
}
