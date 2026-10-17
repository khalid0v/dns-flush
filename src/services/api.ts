/** Typed wrappers around the Tauri backend commands. */
import { invoke } from "@tauri-apps/api/core";
import type { AdapterInfo, LogLine, ScanResult } from "@/types";

export interface StaticIpArgs {
  index: number;
  address: string;
  prefix: number;
  altAddress?: string;
  gateway?: string;
  dnsServers: string[];
}

export interface AutoSetupArgs {
  index: number;
  primaryNet: string;
  altNet: string;
  firstHost: number;
  altStartHost: number;
  autoAlt: boolean;
}

export const api = {
  listAdapters: () => invoke<AdapterInfo[]>("list_adapters"),
  getPublicIp: () => invoke<string>("get_public_ip"),
  flushDns: () => invoke<LogLine[]>("flush_dns"),
  registerRenew: (index: number) =>
    invoke<LogLine[]>("register_renew", { index }),
  resetStack: () => invoke<LogLine[]>("reset_stack"),
  setDns: (index: number, servers: string[]) =>
    invoke<LogLine[]>("set_dns", { index, servers }),
  resetDns: (index: number) => invoke<LogLine[]>("reset_dns", { index }),
  enableDhcp: (index: number) => invoke<LogLine[]>("enable_dhcp", { index }),
  setStaticIp: (args: StaticIpArgs) =>
    invoke<LogLine[]>("set_static_ip", {
      index: args.index,
      address: args.address,
      prefix: args.prefix,
      alt_address: args.altAddress ?? null,
      gateway: args.gateway ?? null,
      dns_servers: args.dnsServers,
    }),
  scanNetwork: (index: number, cidr: string) =>
    invoke<ScanResult>("scan_network", { index, cidr }),
  autoSetup: (args: AutoSetupArgs) =>
    invoke<LogLine[]>("auto_setup", {
      index: args.index,
      primary_net: args.primaryNet,
      alt_net: args.altNet,
      first_host: args.firstHost,
      alt_start_host: args.altStartHost,
      auto_alt: args.autoAlt,
    }),
  setMonitorEnabled: (on: boolean) =>
    invoke<void>("set_monitor_enabled", { on }),
  refreshMonitor: () => invoke<void>("refresh_monitor"),
};
