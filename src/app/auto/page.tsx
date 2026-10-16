import { useState } from "react";
import { ConfirmAction } from "@/app/_components/ConfirmAction";
import { PageHeader } from "@/app/_components/PageHeader";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { usePageProps } from "@/hooks/usePageProps";
import { api } from "@/lib/api";

export default function AutoPage() {
  const { selected, busy, runAction } = usePageProps();
  const [primaryNet, setPrimaryNet] = useState("192.168.1");
  const [altNet, setAltNet] = useState("172.16.0");
  const [firstHost, setFirstHost] = useState("11");
  const [altStartHost, setAltStartHost] = useState("200");
  const [autoAlt, setAutoAlt] = useState(true);

  if (!selected) {
    return (
      <div>
        <PageHeader
          title="Automatic network setup"
          sub="Find an available address and configure this adapter for that network."
        />
        <p className="text-sm text-muted-foreground">Select a network adapter first.</p>
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Automatic network setup"
        sub="Find an available address and configure this adapter for that network."
      />
      <div className="mb-4 grid w-full max-w-md gap-3">
        <div className="grid gap-1.5">
          <Label htmlFor="auto-primary">Primary network (first three octets)</Label>
          <Input
            id="auto-primary"
            className="font-mono"
            value={primaryNet}
            onChange={(e) => setPrimaryNet(e.target.value)}
            placeholder="192.168.1"
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor="auto-alt">Alternate network (first three octets)</Label>
          <Input
            id="auto-alt"
            className="font-mono"
            value={altNet}
            onChange={(e) => setAltNet(e.target.value)}
            placeholder="172.16.0"
          />
        </div>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div className="grid gap-1.5">
            <Label htmlFor="auto-first">First host to try</Label>
            <Input
              id="auto-first"
              inputMode="numeric"
              className="font-mono"
              value={firstHost}
              onChange={(e) => setFirstHost(e.target.value)}
            />
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="auto-alt-start">Alternate start host</Label>
            <Input
              id="auto-alt-start"
              inputMode="numeric"
              className="font-mono"
              value={altStartHost}
              onChange={(e) => setAltStartHost(e.target.value)}
            />
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Switch id="auto-alt-on" checked={autoAlt} onCheckedChange={setAutoAlt} />
          <Label htmlFor="auto-alt-on">Also assign an alternate address</Label>
        </div>
      </div>
      <ConfirmAction
        title="Run automatic setup?"
        description={`Scans ${primaryNet.trim()}.0/24 for the first free address and applies it as a static IP on ${selected.description}. The connection may drop briefly.`}
        actionLabel="Apply"
        disabled={busy}
        onConfirm={() =>
          void runAction(() =>
            api.autoSetup({
              index: selected.index,
              primaryNet: primaryNet.trim(),
              altNet: altNet.trim(),
              firstHost: Number(firstHost) || 0,
              altStartHost: Number(altStartHost) || 0,
              autoAlt,
            }),
          )
        }
      >
        Run automatic setup
      </ConfirmAction>
    </div>
  );
}
