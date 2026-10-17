import { useState } from "react";
import { ConfirmAction } from "@/app/_components/ConfirmAction";
import { PageHeader } from "@/app/_components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { usePageProps } from "@/hooks/usePageProps";
import { currentDnsLabel, isIpv4 } from "@/features/adapters";
import { api } from "@/services/api";

const PRESETS: { name: string; servers: string[] }[] = [
  { name: "Google", servers: ["8.8.8.8", "8.8.4.4"] },
  { name: "Cloudflare", servers: ["1.1.1.1", "1.0.0.1"] },
  { name: "AdGuard", servers: ["94.140.14.14", "94.140.15.15"] },
  { name: "Quad9", servers: ["9.9.9.9", "149.112.112.112"] },
];

export default function DnsPage() {
  const { selected, busy, runAction, pushLog } = usePageProps();
  const [manual, setManual] = useState(false);
  const [dns1, setDns1] = useState("");
  const [dns2, setDns2] = useState("");

  if (!selected) {
    return (
      <div>
        <PageHeader
          title="Choose DNS servers"
          sub="Set static DNS servers on the selected adapter, or return it to automatic."
        />
        <p className="text-sm text-muted-foreground">Select a network adapter first.</p>
      </div>
    );
  }

  const apply = (servers: string[]) => {
    for (const server of servers) {
      if (!isIpv4(server)) {
        pushLog("err", `Invalid DNS server: ${server}`);
        return;
      }
    }
    void runAction(() => api.setDns(selected.index, servers));
  };

  const pending = manual
    ? [dns1.trim(), dns2.trim()].filter((value) => value !== "")
    : [];

  return (
    <div>
      <PageHeader
        title="Choose DNS servers"
        sub="Set static DNS servers on the selected adapter, or return it to automatic."
      />
      <p className="mb-4 text-sm">
        Current DNS: <span className="font-mono">{currentDnsLabel(selected)}</span>
      </p>

      <p className="mb-2 text-sm font-medium">Presets</p>
      <div className="mb-4 grid grid-cols-1 gap-2 sm:grid-cols-2">
        {PRESETS.map((preset) => (
          <Button
            key={preset.name}
            variant="outline"
            disabled={busy}
            onClick={() => {
              setManual(true);
              setDns1(preset.servers[0] ?? "");
              setDns2(preset.servers[1] ?? "");
            }}
          >
            {preset.name} ({preset.servers.join(", ")})
          </Button>
        ))}
      </div>

      <div className="mb-4 flex items-center gap-2">
        <Switch id="dns-manual" checked={manual} onCheckedChange={setManual} />
        <Label htmlFor="dns-manual">Manual servers</Label>
      </div>

      {manual && (
        <div className="mb-4 grid w-full max-w-md gap-3">
          <div className="grid gap-1.5">
            <Label htmlFor="dns1">Preferred DNS server</Label>
            <Input
              id="dns1"
              className="font-mono"
              value={dns1}
              onChange={(e) => setDns1(e.target.value)}
              placeholder="1.1.1.1"
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="dns2">Alternate DNS server (optional)</Label>
            <Input
              id="dns2"
              className="font-mono"
              value={dns2}
              onChange={(e) => setDns2(e.target.value)}
              placeholder="1.0.0.1"
            />
          </div>
        </div>
      )}

      <div className="flex flex-wrap gap-2">
        <ConfirmAction
          title="Apply DNS servers?"
          description={`Set DNS on ${selected.description} to ${pending.length > 0 ? pending.join(", ") : "—"}. Current: ${currentDnsLabel(selected)}.`}
          actionLabel="Apply"
          disabled={busy || !manual || pending.length === 0}
          onConfirm={() => apply(pending)}
        >
          Apply DNS servers
        </ConfirmAction>
        <ConfirmAction
          title="Reset DNS to automatic?"
          description={`Return ${selected.description} to DHCP-provided DNS servers. Current: ${currentDnsLabel(selected)}.`}
          actionLabel="Reset"
          disabled={busy}
          onConfirm={() => void runAction(() => api.resetDns(selected.index))}
        >
          Reset to automatic
        </ConfirmAction>
      </div>
    </div>
  );
}
