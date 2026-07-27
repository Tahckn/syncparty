import { useTranslate } from "@/shared/i18n";
import { useCopy } from "@/shared/hooks/useCopy";
import { Card, CopyRow } from "@/shared/ui";
import type { HostingInfo } from "@/shared/types/HostingInfo";

/**
 * What the host actually sends to people.
 *
 * The link comes first and the raw connection details are secondary: pasting
 * one URL is the path that works, and the rest is there for when it does not.
 */
export function InviteCard({ hosting }: { hosting: HostingInfo }) {
  const t = useTranslate();
  const { copy, copiedKey } = useCopy();

  return (
    <div className="space-y-4">
      <Card title={t("host.invite.title")}>
        <div className="space-y-4">
          <CopyRow
            label={t("host.invite.link")}
            value={hosting.deepLink}
            copyLabel={t("common.copy")}
            copiedLabel={t("common.copied")}
            copied={copiedKey === "link"}
            onCopy={() => void copy("link", hosting.deepLink)}
          />
          <CopyRow
            label={t("host.invite.code")}
            value={hosting.inviteCode}
            copyLabel={t("common.copy")}
            copiedLabel={t("common.copied")}
            copied={copiedKey === "code"}
            onCopy={() => void copy("code", hosting.inviteCode)}
          />
          <p className="text-xs text-ink-faint">{t("host.invite.hint")}</p>
        </div>
      </Card>

      <Card title={t("host.details.title")}>
        <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-2">
          <Detail
            label={t("host.details.server")}
            value={`${hosting.invite.host}:${hosting.invite.port}`}
          />
          <Detail label={t("host.details.room")} value={hosting.invite.room} />
          <Detail
            label={t("host.details.password")}
            value={hosting.invite.password}
          />
          <Detail
            label={t("host.details.tailscale")}
            value={hosting.tailscaleAddress}
          />
        </dl>
      </Card>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs font-medium tracking-wide text-ink-faint uppercase">
        {label}
      </dt>
      <dd className="selectable mt-0.5 truncate font-mono text-sm text-ink">
        {value}
      </dd>
    </div>
  );
}
