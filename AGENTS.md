# AGENTS.md

Guidance for AI coding agents (and humans) working in this repo. Read it fully
before changing anything.

## 1. Project overview

`dns` is a **Windows-only** network-troubleshooting tool, shipped as a single
Rust crate:

| Part                    | Status              | Role                                                                 |
| ----------------------- | ------------------- | -------------------------------------------------------------------- |
| Rust crate `dns`        | **Primary product** | `eframe`/`egui` desktop GUI with 8 network-troubleshooting actions  |

The Rust app is an Apple HIG-inspired (macOS-style) GUI with light/dark modes.
It self-elevates to Administrator at startup (UAC) because most actions change
network state.

## 2. Build: use the GNU toolchain (MSVC linker is missing)

The dev machine has `rustc`/`cargo` 1.99 but **no `link.exe`**; only MinGW gcc
(`C:\msys64\mingw64\bin`) is installed. The MSVC toolchain therefore cannot link
build scripts/proc-macros. Build/test/lint with the GNU toolchain:

```bash
cargo +stable-x86_64-pc-windows-gnu build
cargo +stable-x86_64-pc-windows-gnu clippy --all-targets -- -D warnings
cargo +stable-x86_64-pc-windows-gnu fmt --check
cargo +stable-x86_64-pc-windows-gnu test
```

Plain `cargo build` fails with `linker link.exe not found`. The GNU target and
toolchain are already installed. (Alternatively set a rustup override for this
directory.) `Cargo.lock` is committed (binary crate).

## 3. Architecture

```
src/
├── main.rs        # elevation check + eframe entrypoint (no logic)
├── app.rs         # egui App: layout, forms, action dispatch, worker threads
├── theme.rs       # Apple HIG visuals (light/dark) + fonts
├── elevate.rs     # IsUserAnAdmin / ShellExecuteW "runas" relaunch
├── msg.rs         # Msg enum + Tx: worker -> UI channel
└── win/           # all OS access; nothing here may touch egui
    ├── mod.rs     # exec_method (WMI mid-level helper), return_code
    ├── adapters.rs# Win32_NetworkAdapterConfiguration enumeration (serde)
    ├── dns.rs     # flush (runtime dnsapi lookup), resolve, set/reset DNS
    ├── ip.rs      # static IP, DHCP, release/renew, mask parsing
    ├── net.rs     # ICMP ping via IcmpSendEcho
    ├── reset.rs   # netsh / ipconfig fallbacks
    └── scan.rs    # parallel ping sweep + GetIpNetTable (ARP)
```

Long-running actions run on a `std::thread` and stream `Msg::Log` back over an
`mpsc` channel; `DnsApp::poll` drains it each frame. `WMIConnection` is not
`Send`, so each worker creates its own connection inside the thread. Do not
share a connection across threads. `DnsApp::spawn` always sends `Msg::Done` last
(that clears the busy flag).

## 4. Non-obvious Windows/egui gotchas (hard-won)

- **egui/eframe 0.36 API differs from older tutorials.** `eframe::App` requires
  `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)` — there is no
  `update(ctx, frame)`. Panels are `egui::Panel::{top,bottom,left,right}(id)` and
  `show(ui, ..)` takes a `Ui`, not a `Context`. Use `ctx.all_styles_mut(...)` for
  styles and `ctx.set_visuals(...)` for colors.
- **WMI array arguments:** the high-level `WMIConnection::exec_instance_method`
  cannot pass array parameters. Use `win::exec_method` (mid-level) with
  `Variant::Array(vec![Variant::String(..)])` for `EnableStatic`,
  `SetGateways`, `SetDNSServerSearchOrder`. Resetting DNS = empty array (maps to
  `VT_NULL`).
- **`DnsFlushResolverCache` is not in MinGW's import library.** It is loaded at
  runtime via `LoadLibraryA`/`GetProcAddress` in `win/dns.rs`. Do not re-add a
  `#[link(name = "dnsapi")]` extern for it.
- **Actions 2 and 3 have no public Win32 API.** DNS registration
  (`ipconfig /registerdns`) and Winsock/TCP-IP reset (`netsh winsock reset`,
  `netsh int ip reset`) shell out in `win/reset.rs`. Everything else uses native
  APIs / WMI.
- ICMP addresses are passed in network byte order: build the `u32` with
  `u32::from_ne_bytes(ip.octets())` (see `win/net.rs`), not `u32::from(ip)`.
- WMI property names are matched exactly by serde (`IPEnabled`, `IPAddress`,
  `DefaultIPGateway`, `DNSServerSearchOrder`, `DHCPEnabled`, ...). Keep the
  explicit `#[serde(rename = "...")]` casing.

## 5. Safety rules for agents

Network changes can lock a machine out of the network.

1. Never run, or tell the user to run, state-changing operations (static IP, DNS
   change, adapter reset) without explicit approval. The GUI must keep the
   confirm step for the reset action.
2. Every state-changing action must show the current config, the planned change,
   and ask for confirmation.
3. Scan only networks the user owns or administers.
4. Never commit real IPs, hostnames, or credentials.

## 6. Definition of done

- `cargo +stable-x86_64-pc-windows-gnu fmt --check`, `clippy --all-targets --
  -D warnings`, and `test` pass.
- No `unwrap()`/`expect()` outside tests; `win/` never imports `egui`; no I/O in
  `theme.rs`/`msg.rs`.
- Manually verified on Windows **as Administrator** for any change touching
  `win/` (do not claim it works otherwise).
