use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ipnet::Ipv4Net;
use windows_sys::Win32::NetworkManagement::IpHelper::{GetIpNetTable, MIB_IPNETTABLE};

use super::adapters::AdapterInfo;
use super::net::{ping, u32_to_ip};
use crate::msg::{ScanDevice, ScanResult};

pub fn parse_cidr(text: &str) -> Option<Ipv4Net> {
    let net: Ipv4Net = text.trim().parse().ok()?;
    Ipv4Net::new(net.network(), net.prefix_len()).ok()
}

fn arp_table(interface_index: u32) -> Vec<(Ipv4Addr, String)> {
    unsafe {
        let mut size: u32 = 0;
        let _ = GetIpNetTable(std::ptr::null_mut(), &mut size, 0);
        if size == 0 {
            return Vec::new();
        }

        let mut buf = vec![0u8; size as usize];
        let table = buf.as_mut_ptr() as *mut MIB_IPNETTABLE;
        if GetIpNetTable(table, &mut size, 0) != 0 {
            return Vec::new();
        }

        let count = (*table).dwNumEntries as usize;
        let rows = std::slice::from_raw_parts((*table).table.as_ptr(), count);

        let mut out = Vec::new();
        for row in rows {
            if row.dwIndex != interface_index {
                continue;
            }
            if row.bPhysAddr[..row.dwPhysAddrLen as usize]
                .iter()
                .all(|b| *b == 0)
            {
                continue;
            }
            if row.bPhysAddr[0] & 0x01 != 0 {
                continue;
            }
            let mac = row.bPhysAddr[..(row.dwPhysAddrLen as usize).min(6)]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join("-");
            out.push((u32_to_ip(row.dwAddr), mac));
        }
        out
    }
}

/// Ping-sweep a subnet in parallel and merge in the ARP table so devices that
/// block ICMP but answer ARP are still detected.
pub fn scan_net(interface_index: u32, net: Ipv4Net, timeout_ms: u32) -> HashMap<Ipv4Addr, String> {
    let hosts: Vec<Ipv4Addr> = net.hosts().collect();
    let used: Mutex<HashMap<Ipv4Addr, String>> = Mutex::new(HashMap::new());
    let next = AtomicUsize::new(0);
    let workers = hosts.len().clamp(1, 64);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            let used = &used;
            let next = &next;
            let hosts = &hosts;
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(ip) = hosts.get(i) else { break };
                    if ping(*ip, timeout_ms).is_some() {
                        used.lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .insert(*ip, "-".to_string());
                    }
                }
            });
        }
    });

    let mut used = used.into_inner().unwrap_or_default();
    for (ip, mac) in arp_table(interface_index) {
        if net.contains(&ip) {
            used.entry(ip).or_insert(mac);
        }
    }
    used
}

/// Scan `net` and return the devices in use plus the free host addresses.
pub fn scan_result(interface_index: u32, net: Ipv4Net, ad: &AdapterInfo) -> ScanResult {
    let used_map = scan_net(interface_index, net, 700);
    let hosts: Vec<Ipv4Addr> = net.hosts().collect();
    let local = ad.primary_ip().and_then(|s| s.parse::<Ipv4Addr>().ok());

    let mut used: Vec<ScanDevice> = used_map
        .iter()
        .map(|(ip, mac)| ScanDevice {
            ip: *ip,
            mac: mac.clone(),
            note: if Some(*ip) == local {
                "this PC".to_string()
            } else if ad.gateways.iter().any(|g| g == &ip.to_string()) {
                "gateway".to_string()
            } else {
                String::new()
            },
        })
        .collect();
    used.sort_by_key(|d| u32::from(d.ip));

    let free: Vec<Ipv4Addr> = hosts
        .iter()
        .copied()
        .filter(|h| !used_map.contains_key(h))
        .collect();

    ScanResult {
        network: net.to_string(),
        used,
        free,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_cidr_to_network() {
        let net = parse_cidr("192.168.1.37/24").expect("valid CIDR");
        assert_eq!(net.network().to_string(), "192.168.1.0");
        assert_eq!(net.prefix_len(), 24);
    }

    #[test]
    fn accepts_network_address_without_host_bits() {
        let net = parse_cidr("10.0.0.0/8").expect("valid CIDR");
        assert_eq!(net.network().to_string(), "10.0.0.0");
    }

    #[test]
    fn rejects_malformed_cidr() {
        assert!(parse_cidr("192.168.1.0").is_none());
        assert!(parse_cidr("nonsense").is_none());
    }
}
