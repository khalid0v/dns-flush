import { useEffect, useState } from "react";
import { PageHeader } from "@/components/app/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { primaryCidr, primaryIp, subnetFor } from "@/lib/adapters";
import { api } from "@/lib/api";
import type { PageProps, ScanResult } from "@/lib/types";

export function ScanPage({ selected, busy, runAction, pushLog }: PageProps) {
  const [cidr, setCidr] = useState("");
  // WMI index the range box was derived from. Mirrors the backend guard: a
  // new range is only suggested when the selection moves to another adapter,
  // so hand-typed ranges and finished scans survive refreshes.
  const [cidrFor, setCidrFor] = useState<number | null>(null);
  const [result, setResult] = useState<ScanResult | null>(null);

  const index = selected?.index ?? null;
  const recommendation = selected ? primaryCidr(selected) : undefined;
  const thisIp = selected ? primaryIp(selected) : undefined;
  const thisMask =
    selected && thisIp ? (subnetFor(selected, thisIp) ?? "auto /24") : "unknown";

  useEffect(() => {
    if (index !== cidrFor) {
      setCidrFor(index);
      setResult(null);
      if (recommendation) setCidr(recommendation);
    }
  }, [index, cidrFor, recommendation]);

  const find = () => {
    if (selected === undefined) {
      pushLog("err", "No adapter selected.");
      return;
    }
    const adapterIndex = selected.index;
    const range = cidr.trim();
    pushLog("head", `[7] Scan ${range}`);
    void runAction(async () => {
      setResult(await api.scanNetwork(adapterIndex, range));
    });
  };

  return (
    <div>
      <PageHeader
        title="Find devices on your network"
        sub="Check a network you own or administer to see which addresses are in use."
      />

      {selected && (
        <p className="mb-1 text-sm text-muted-foreground">
          This PC: {thisIp ?? "no IP"} · subnet {thisMask} ·{" "}
          {selected.dhcp_enabled ? "DHCP" : "Static"} · {selected.description}
        </p>
      )}
      {recommendation ? (
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <p className="text-sm text-muted-foreground">
            Recommended: <span className="font-mono">{recommendation}</span> (from this
            PC&apos;s address)
          </p>
          {cidr !== recommendation && (
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={() => {
                setCidr(recommendation);
                setResult(null);
              }}
            >
              Use recommended
            </Button>
          )}
        </div>
      ) : (
        <p className="mb-3 text-sm text-amber-600 dark:text-amber-400">
          The selected adapter has no usable address, so no range can be recommended.
        </p>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <Label htmlFor="scan-range">Network range</Label>
        <Input
          id="scan-range"
          className="w-56 font-mono"
          value={cidr}
          onChange={(e) => setCidr(e.target.value)}
          placeholder="192.168.1.0/24"
        />
        <Button disabled={busy} onClick={find}>
          Find devices
        </Button>
      </div>

      {result === null ? (
        <p className="mt-4 text-sm text-muted-foreground">
          Choose a range and press Find devices to see what is in use.
        </p>
      ) : (
        <div className="mt-4">
          <p className="text-sm font-semibold">
            {result.used.length} of {result.used.length + result.free.length} addresses in
            use in {result.network}
          </p>
          <div className="mt-2 rounded-md border">
            <ScrollArea className="h-60">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>IP address</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>MAC / note</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {result.used.map((device) => (
                    <TableRow key={device.ip} className="even:bg-muted/50">
                      <TableCell className="font-mono">{device.ip}</TableCell>
                      <TableCell>
                        <Badge variant="secondary">In use</Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {`${device.mac} ${device.note}`.trim() || "—"}
                      </TableCell>
                    </TableRow>
                  ))}
                  {result.free.map((ip) => (
                    <TableRow key={ip} className="even:bg-muted/50">
                      <TableCell className="font-mono">{ip}</TableCell>
                      <TableCell>
                        <Badge variant="outline">Free</Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">—</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </ScrollArea>
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            Observations only — a free address may be taken by a device that did not
            answer.
          </p>
        </div>
      )}
    </div>
  );
}
