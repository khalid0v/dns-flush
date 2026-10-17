/**
 * Pure adapter helpers, ported from the Rust core (`win/adapters.rs`,
 * `win/ip.rs`, `monitor.rs`) so the UI can derive display values without a
 * backend round-trip. No I/O here.
 */
import type { AdapterInfo } from "./types";

const APIPA_PREFIX = "169.254.";

const VPN_MARKERS = [
  "vpn",
  "wireguard",
  "openvpn",
  "wintun",
  "tap-windows",
  "fortinet",
  "forticlient",
  "globalprotect",
  "anyconnect",
  "tailscale",
  "nordlynx",
  "protonvpn",
  "mullvad",
  "zerotier",
  "pulse secure",
];

export function adapterTitle(a: AdapterInfo): string {
  const ip = a.ip_addresses[0] ?? "no ip";
  return `#${a.index}  ${a.description}  [${ip}]`;
}

export function primaryIp(a: AdapterInfo): string | undefined {
  return a.ip_addresses.find((address) => !address.startsWith(APIPA_PREFIX));
}

export function subnetFor(a: AdapterInfo, ip: string): string | undefined {
  const at = a.ip_addresses.indexOf(ip);
  return at >= 0 ? a.ip_subnets[at] : undefined;
}

/** Parse a dotted mask or a bare prefix length into a prefix length. */
export function parseMask(text: string): number | undefined {
  const trimmed = text.trim();
  if (/^\d{1,2}$/.test(trimmed)) {
    const prefix = Number(trimmed);
    return prefix <= 32 ? prefix : undefined;
  }
  const parts = trimmed.split(".");
  if (parts.length !== 4) return undefined;
  const nums = parts.map(Number);
  if (nums.some((n) => !Number.isInteger(n) || n < 0 || n > 255)) return undefined;
  let bits = ((nums[0] * 256 + nums[1]) * 256 + nums[2]) * 256 + nums[3];
  bits >>>= 0;
  let prefix = 0;
  while (prefix < 32 && (bits & 0x80000000) !== 0) {
    prefix += 1;
    bits = (bits << 1) >>> 0;
  }
  return bits === 0 ? prefix : undefined;
}

function dotted(addr: number): string {
  const v = addr >>> 0;
  return `${(v >>> 24) & 255}.${(v >>> 16) & 255}.${(v >>> 8) & 255}.${v & 255}`;
}

function parseIpv4(text: string): number | undefined {
  const parts = text.split(".");
  if (parts.length !== 4) return undefined;
  const nums = parts.map(Number);
  if (nums.some((n) => !Number.isInteger(n) || n < 0 || n > 255)) return undefined;
  return (((nums[0] * 256 + nums[1]) * 256 + nums[2]) * 256 + nums[3]) >>> 0;
}

/**
 * CIDR of the adapter's primary (non-APIPA) address, e.g. `172.16.0.0/24`.
 * Uses the configured mask when present, else a /24 on the address's network.
 */
export function primaryCidr(a: AdapterInfo): string | undefined {
  const ip = primaryIp(a);
  if (!ip) return undefined;
  const addr = parseIpv4(ip);
  if (addr === undefined) return undefined;
  const mask = subnetFor(a, ip);
  const prefix = mask ? (parseMask(mask) ?? 24) : 24;
  const net = prefix === 0 ? 0 : (((addr >>> (32 - prefix)) << (32 - prefix)) >>> 0);
  return `${dotted(net)}/${prefix}`;
}

/**
 * Position of the adapter most likely to be live: the first with a real
 * address and a gateway, else the first with a real address, else first.
 */
export function preferredAdapter(adapters: AdapterInfo[]): number {
  const withGateway = adapters.findIndex(
    (a) => primaryIp(a) !== undefined && a.gateways.length > 0,
  );
  if (withGateway >= 0) return withGateway;
  const withIp = adapters.findIndex((a) => primaryIp(a) !== undefined);
  return withIp >= 0 ? withIp : 0;
}

function isUsableIpv4(addr: number): boolean {
  const v = addr >>> 0;
  const b0 = (v >>> 24) & 255;
  const b1 = (v >>> 16) & 255;
  if (v === 0 || v === 0xffffffff) return false;
  if (b0 === 127) return false; // loopback
  if (b0 >= 224 && b0 <= 239) return false; // multicast
  if (b0 === 169 && b1 === 254) return false; // APIPA
  return true;
}

/** Roughly mirrors `monitor::is_usable_ip`: drops unspecified, loopback, multicast, and link-local addresses. */
export function isUsableIp(text: string): boolean {
  if (text.includes(":")) {
    const lower = text.toLowerCase();
    if (lower === "::" || lower === "::1") return false;
    if (lower.startsWith("ff")) return false;
    if (lower.startsWith("fe80")) return false;
    return true;
  }
  const addr = parseIpv4(text);
  return addr !== undefined && isUsableIpv4(addr);
}

export function currentDnsLabel(a: AdapterInfo): string {
  if (a.dns_servers.length === 0) {
    return a.dhcp_enabled ? "Automatic (DHCP)" : "Not set";
  }
  return a.dns_servers.join(", ");
}

export function currentIpLabel(a: AdapterInfo): string {
  const usable = a.ip_addresses.filter(isUsableIp);
  return usable.length > 0 ? usable.join(", ") : "No active IP address";
}

function isVpn(a: AdapterInfo): boolean {
  const text = `${a.description} ${adapterTitle(a)}`.toLowerCase();
  return VPN_MARKERS.some((marker) => text.includes(marker));
}

/** Human-readable summary of the active VPN, if any. Heuristic: name markers plus a usable address. */
export function vpnStatusLabel(adapters: AdapterInfo[]): string {
  const active = adapters.find(
    (a) => isVpn(a) && a.ip_addresses.some(isUsableIp),
  );
  return active
    ? `Detected (${active.description})`
    : "No known VPN adapter detected";
}

/** Strict dotted-quad IPv4 check for form validation. */
export function isIpv4(text: string): boolean {
  const parts = text.trim().split(".");
  if (parts.length !== 4) return false;
  return parts.every((part) => {
    if (!/^\d{1,3}$/.test(part)) return false;
    const n = Number(part);
    return n >= 0 && n <= 255;
  });
}
