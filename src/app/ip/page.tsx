import { useState } from "react";
import { ConfirmAction } from "@/app/_components/ConfirmAction";
import { PageHeader } from "@/app/_components/PageHeader";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { usePageProps } from "@/hooks/usePageProps";
import { currentIpLabel, isIpv4, parseMask } from "@/lib/adapters";
import { api } from "@/lib/api";

export default function IpPage() {
  const { selected, busy, runAction, pushLog } = usePageProps();
  const [useStatic, setUseStatic] = useState(true);
  const [address, setAddress] = useState("");
  const [mask, setMask] = useState("255.255.255.0");
  const [gateway, setGateway] = useState("");
  const [dns1, setDns1] = useState("");
  const [dns2, setDns2] = useState("");
  const [altAddress, setAltAddress] = useState("");

  if (!selected) {
    return (
      <div>
        <PageHeader
          title="Set IP address"
          sub="Switch the selected adapter between DHCP and a static address."
        />
        <p className="text-sm text-muted-foreground">Select a network adapter first.</p>
      </div>
    );
  }

  const applyStatic = () => {
    const trimmed = address.trim();
    if (!isIpv4(trimmed)) {
      pushLog("err", `Invalid IP address: ${trimmed}`);
      return;
    }
    const prefix = parseMask(mask);
    if (prefix === undefined) {
      pushLog("err", `Invalid mask: ${mask.trim()}`);
      return;
    }
    const alt = altAddress.trim();
    if (alt !== "" && !isIpv4(alt)) {
      pushLog("err", `Invalid alternate IP: ${alt}`);
      return;
    }
    const gw = gateway.trim();
    if (gw !== "" && !isIpv4(gw)) {
      pushLog("err", `Invalid gateway: ${gw}`);
      return;
    }
    const dns = [dns1.trim(), dns2.trim()].filter((value) => value !== "");
    for (const server of dns) {
      if (!isIpv4(server)) {
        pushLog("err", `Invalid DNS server: ${server}`);
        return;
      }
    }
    void runAction(() =>
      api.setStaticIp({
        index: selected.index,
        address: trimmed,
        prefix,
        altAddress: alt === "" ? undefined : alt,
        gateway: gw === "" ? undefined : gw,
        dnsServers: dns,
      }),
    );
  };

  const field = (
    id: string,
    label: string,
    value: string,
    setValue: (value: string) => void,
    placeholder: string,
  ) => (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        className="font-mono"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder}
      />
    </div>
  );

  return (
    <div>
      <PageHeader
        title="Set IP address"
        sub="Switch the selected adapter between DHCP and a static address."
      />
      <p className="mb-4 text-sm">
        Current: <span className="font-mono">{currentIpLabel(selected)}</span>
      </p>

      <div className="mb-4 flex items-center gap-2">
        <Switch id="ip-static" checked={useStatic} onCheckedChange={setUseStatic} />
        <Label htmlFor="ip-static">Manual (static) address</Label>
      </div>

      {!useStatic ? (
        <ConfirmAction
          title="Switch to DHCP?"
          description={`Enable DHCP on ${selected.description}. The current static address, if any, will be released. Current: ${currentIpLabel(selected)}.`}
          actionLabel="Switch"
          disabled={busy}
          onConfirm={() => void runAction(() => api.enableDhcp(selected.index))}
        >
          Switch to DHCP
        </ConfirmAction>
      ) : (
        <div>
          <div className="mb-4 grid w-full max-w-md gap-3">
            {field("ip-address", "IP address", address, setAddress, "192.168.1.20")}
            {field("ip-mask", "Subnet mask or prefix", mask, setMask, "255.255.255.0 or 24")}
            {field("ip-gateway", "Gateway (optional)", gateway, setGateway, "192.168.1.1")}
            {field("ip-dns1", "Preferred DNS (optional)", dns1, setDns1, "1.1.1.1")}
            {field("ip-dns2", "Alternate DNS (optional)", dns2, setDns2, "1.0.0.1")}
            {field("ip-alt", "Alternate IP (optional)", altAddress, setAltAddress, "")}
          </div>
          <ConfirmAction
            title="Apply static IP?"
            description={`Apply ${address.trim() || "—"}/${mask.trim() || "—"}${gateway.trim() ? `, gateway ${gateway.trim()}` : ""} on ${selected.description}. The connection may drop briefly. Current: ${currentIpLabel(selected)}.`}
            actionLabel="Apply"
            disabled={busy}
            onConfirm={applyStatic}
          >
            Apply IP settings
          </ConfirmAction>
        </div>
      )}
    </div>
  );
}
