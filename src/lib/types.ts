/** Shared shapes mirroring the Tauri backend's serde output. */

export interface AdapterInfo {
  index: number;
  interface_index: number;
  description: string;
  ip_addresses: string[];
  ip_subnets: string[];
  gateways: string[];
  dns_servers: string[];
  dhcp_enabled: boolean;
}

export interface ScanDevice {
  ip: string;
  mac: string;
  note: string;
}

export interface ScanResult {
  network: string;
  used: ScanDevice[];
  free: string[];
}

export type LogKind = "info" | "ok" | "warn" | "err" | "head";

export interface LogLine {
  kind: LogKind;
  text: string;
}

export interface PublicIpPayload {
  ok: boolean;
  value: string;
}

export type PageKey =
  | "status"
  | "scan"
  | "flush"
  | "renew"
  | "reset"
  | "dns"
  | "ip"
  | "auto";

/** Props every tool page receives from the app shell. */
export interface PageProps {
  adapters: AdapterInfo[];
  selected: AdapterInfo | undefined;
  busy: boolean;
  runAction: (fn: () => Promise<unknown>) => Promise<void>;
  pushLog: (kind: LogKind, text: string) => void;
}
