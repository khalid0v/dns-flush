//! Pure helpers that turn read-only adapter observations into human-readable
//! change descriptions. No OS access and no egui here so it can be unit tested.

use std::net::IpAddr;

use crate::win::AdapterInfo;

/// How an adapter appears to the user, inferred from its name/description.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdapterKind {
    Wifi,
    Ethernet,
    Vpn,
    Other,
}

impl AdapterKind {
    pub fn label(self) -> &'static str {
        match self {
            AdapterKind::Wifi => "Wi-Fi",
            AdapterKind::Ethernet => "Ethernet",
            AdapterKind::Vpn => "VPN",
            AdapterKind::Other => "network",
        }
    }
}

const VPN_MARKERS: [&str; 15] = [
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

const WIFI_MARKERS: [&str; 5] = ["wi-fi", "wifi", "wireless", "wlan", "802.11"];

/// Classify an adapter by matching its description (and title) for known
/// markers. This is a heuristic: some devices may not be recognized.
pub fn classify_adapter(adapter: &AdapterInfo) -> AdapterKind {
    let text = format!("{} {}", adapter.description, adapter.title()).to_ascii_lowercase();
    if VPN_MARKERS.iter().any(|marker| text.contains(marker)) {
        AdapterKind::Vpn
    } else if WIFI_MARKERS.iter().any(|marker| text.contains(marker)) {
        AdapterKind::Wifi
    } else if text.contains("ethernet") || text.contains("local area connection") {
        AdapterKind::Ethernet
    } else {
        AdapterKind::Other
    }
}

/// Human-readable summary of the active VPN, if any.
pub fn vpn_status_label(adapters: &[AdapterInfo]) -> String {
    let active = adapters.iter().find(|adapter| {
        classify_adapter(adapter) == AdapterKind::Vpn
            && adapter
                .ip_addresses
                .iter()
                .any(|address| address.parse::<IpAddr>().is_ok_and(is_usable_ip))
    });

    active
        .map(|adapter| format!("Detected ({})", adapter.description))
        .unwrap_or_else(|| "No known VPN adapter detected".to_string())
}

pub fn is_usable_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !(address.octets()[0] == 169 && address.octets()[1] == 254)
        }
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_loopback()
                && !address.is_multicast()
                && !address.is_unicast_link_local()
        }
    }
}

fn usable_addresses(adapter: &AdapterInfo) -> Vec<String> {
    adapter
        .ip_addresses
        .iter()
        .filter(|address| address.parse::<IpAddr>().is_ok_and(is_usable_ip))
        .cloned()
        .collect()
}

fn vpn_is_active(adapters: &[AdapterInfo]) -> bool {
    adapters.iter().any(|adapter| {
        classify_adapter(adapter) == AdapterKind::Vpn && !usable_addresses(adapter).is_empty()
    })
}

/// Describe the differences between two adapter snapshots as log lines.
/// Returns an empty vector when nothing meaningful changed. The first
/// snapshot is treated as a baseline by the caller and never diffed.
pub fn describe_changes(previous: &[AdapterInfo], current: &[AdapterInfo]) -> Vec<String> {
    let mut changes = Vec::new();

    let vpn_before = vpn_is_active(previous);
    let vpn_after = vpn_is_active(current);
    if vpn_before != vpn_after {
        changes.push(if vpn_after {
            "VPN connected".to_string()
        } else {
            "VPN disconnected".to_string()
        });
    }

    for old in previous {
        let Some(new) = current.iter().find(|a| a.index == old.index) else {
            changes.push(format!(
                "{} disconnected ({})",
                classify_adapter(old).label(),
                old.description
            ));
            continue;
        };

        let kind = classify_adapter(new).label();
        let old_ips = usable_addresses(old);
        let new_ips = usable_addresses(new);
        if old_ips != new_ips {
            changes.push(format!(
                "{kind} IP changed: {} -> {}",
                join_or_none(&old_ips),
                join_or_none(&new_ips)
            ));
        }

        if old.dns_servers != new.dns_servers {
            changes.push(format!(
                "{kind} DNS changed: {} -> {}",
                join_or_none(&old.dns_servers),
                join_or_none(&new.dns_servers)
            ));
        }
    }

    for new in current {
        if !previous.iter().any(|old| old.index == new.index) {
            changes.push(format!(
                "{} connected ({})",
                classify_adapter(new).label(),
                new.description
            ));
        }
    }

    changes
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

/// One-line toast summary of a set of changes.
pub fn summarize(changes: &[String]) -> String {
    match changes.len() {
        0 => String::new(),
        1 => format!("Network changed — {}", changes[0]),
        _ => format!(
            "Network changed — {} and {} more",
            changes[0],
            changes.len() - 1
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(index: u32, description: &str, ips: &[&str], dns: &[&str]) -> AdapterInfo {
        AdapterInfo {
            index,
            interface_index: index,
            description: description.to_string(),
            ip_addresses: ips.iter().map(|s| (*s).to_string()).collect(),
            ip_subnets: Vec::new(),
            gateways: Vec::new(),
            dns_servers: dns.iter().map(|s| (*s).to_string()).collect(),
            dhcp_enabled: true,
        }
    }

    #[test]
    fn classifies_known_adapter_kinds() {
        assert_eq!(
            classify_adapter(&adapter(1, "Intel Wi-Fi 6 AX200", &[], &[])),
            AdapterKind::Wifi
        );
        assert_eq!(
            classify_adapter(&adapter(2, "Realtek PCIe GbE Family Controller", &[], &[])),
            AdapterKind::Other
        );
        assert_eq!(
            classify_adapter(&adapter(3, "Ethernet", &[], &[])),
            AdapterKind::Ethernet
        );
        assert_eq!(
            classify_adapter(&adapter(4, "WireGuard Tunnel", &[], &[])),
            AdapterKind::Vpn
        );
    }

    #[test]
    fn no_change_is_reported() {
        let a = vec![adapter(1, "Ethernet", &["192.168.1.5"], &["8.8.8.8"])];
        let b = vec![adapter(1, "Ethernet", &["192.168.1.5"], &["8.8.8.8"])];
        assert!(describe_changes(&a, &b).is_empty());
    }

    #[test]
    fn detects_ip_and_dns_changes() {
        let before = vec![adapter(1, "Ethernet", &["192.168.1.5"], &["8.8.8.8"])];
        let after = vec![adapter(1, "Ethernet", &["192.168.1.9"], &["1.1.1.1"])];
        let changes = describe_changes(&before, &after);
        assert!(changes.iter().any(|c| c.contains("IP changed")));
        assert!(changes.iter().any(|c| c.contains("DNS changed")));
    }

    #[test]
    fn detects_adapter_add_and_remove() {
        let before = vec![adapter(1, "Ethernet", &["192.168.1.5"], &[])];
        let after = vec![
            adapter(1, "Ethernet", &["192.168.1.5"], &[]),
            adapter(7, "WireGuard Tunnel", &["10.0.0.2"], &[]),
        ];
        let changes = describe_changes(&before, &after);
        assert!(changes.iter().any(|c| c.contains("VPN connected")));
        assert!(changes.iter().any(|c| c.contains("connected")));

        let removed = describe_changes(&after, &before);
        assert!(removed.iter().any(|c| c.contains("disconnected")));
        assert!(removed.iter().any(|c| c.contains("VPN disconnected")));
    }

    #[test]
    fn summarizes_change_count() {
        assert_eq!(summarize(&[]), "");
        assert_eq!(
            summarize(&["DNS changed".into()]),
            "Network changed — DNS changed"
        );
        assert_eq!(
            summarize(&["VPN connected".into(), "DNS changed".into()]),
            "Network changed — VPN connected and 1 more"
        );
    }
}
