import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageHeader } from "@/app/_components/PageHeader";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { usePageProps } from "@/hooks/usePageProps";
import {
  currentDnsLabel,
  currentIpLabel,
  vpnStatusLabel,
} from "@/features/adapters";
import { api } from "@/services/api";
import { useAppStore } from "@/store/appStore";

function FactCard({ title, value, note }: { title: string; value: string; note: string }) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-lg font-semibold break-words">{value}</p>
        <p className="mt-1 text-sm text-muted-foreground">{note}</p>
      </CardContent>
    </Card>
  );
}

export default function StatusPage() {
  const { adapters, selected, busy, runAction, pushLog } = usePageProps();
  const publicIp = useAppStore((state) => state.publicIp);
  const publicIpLoading = useAppStore((state) => state.publicIpLoading);
  const monitorOn = useAppStore((state) => state.monitorOn);
  const setMonitorEnabled = useAppStore((state) => state.setMonitorEnabled);

  const refresh = () => {
    void runAction(async () => {
      try {
        await api.refreshMonitor();
      } catch (error) {
        pushLog("err", String(error));
      }
    });
  };

  return (
    <div>
      <PageHeader
        title="Network status"
        sub="Current details for the selected network adapter."
      />

      <div className="mb-4 flex items-center gap-2">
        <Switch
          id="live-status"
          checked={monitorOn}
          onCheckedChange={(on) => {
            setMonitorEnabled(on);
            if (on) refresh();
          }}
        />
        <label htmlFor="live-status" className="text-sm font-medium">
          Live status
        </label>
        <span className="text-sm text-muted-foreground">
          {monitorOn ? "Monitoring for changes" : "Paused"}
        </span>
      </div>

      {selected === undefined ? (
        <p className="text-sm text-muted-foreground">
          No adapter selected. Adapters appear here once enumeration finishes.
        </p>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 2xl:grid-cols-4">
          <FactCard
            title="Current DNS"
            value={currentDnsLabel(selected)}
            note="For the selected adapter"
          />
          <FactCard
            title="Current IP address"
            value={currentIpLabel(selected)}
            note="For the selected adapter"
          />
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium text-muted-foreground">
                Public IP address
              </CardTitle>
            </CardHeader>
            <CardContent>
              {publicIpLoading || busy ? (
                <Skeleton className="h-7 w-40" />
              ) : (
                <p className="text-lg font-semibold break-words">
                  {publicIp?.ok === true ? publicIp.value : "Unavailable"}
                </p>
              )}
              <p className="mt-1 text-sm text-muted-foreground">
                External address as seen from the internet.
              </p>
            </CardContent>
          </Card>
          <FactCard
            title="VPN status"
            value={vpnStatusLabel(adapters)}
            note="Based on active adapter names; some VPNs may not be recognized."
          />
        </div>
      )}
    </div>
  );
}
