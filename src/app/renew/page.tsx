import { ConfirmAction } from "@/app/_components/ConfirmAction";
import { PageHeader } from "@/app/_components/PageHeader";
import { usePageProps } from "@/hooks/usePageProps";
import { api } from "@/lib/api";

export default function RenewPage() {
  const { selected, busy, runAction } = usePageProps();

  if (!selected) {
    return (
      <div>
        <PageHeader
          title="Renew your connection"
          sub="Refresh the network address and register this PC with DNS."
        />
        <p className="text-sm text-muted-foreground">Select a network adapter first.</p>
      </div>
    );
  }
  return (
    <div>
      <PageHeader
        title="Renew your connection"
        sub="Refresh the network address and register this PC with DNS."
      />
      <ConfirmAction
        title="Renew connection?"
        description={`Releases and renews the DHCP lease and re-registers DNS on ${selected.description}. The connection may drop briefly.`}
        actionLabel="Renew"
        disabled={busy}
        onConfirm={() => void runAction(() => api.registerRenew(selected.index))}
      >
        Renew connection
      </ConfirmAction>
    </div>
  );
}
