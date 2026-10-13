# AGENTS.md

Guidance for AI coding agents (and humans) working in this repo. Read it fully
before changing anything.

## 1. Project overview

`dns` is a **Windows-only IT support toolkit**, shipped as a single Rust crate.
Its current feature set focuses on network troubleshooting; the product direction
is to cover common Windows endpoint support tasks with guided diagnostics and
carefully confirmed repairs:

| Part                    | Status              | Role                                                                 |
| ----------------------- | ------------------- | -------------------------------------------------------------------- |
| Rust crate `dns`        | **Primary product** | `eframe`/`egui` desktop GUI; currently 7 network-troubleshooting actions |

The Rust app is an Apple HIG-inspired (macOS-style) GUI with light/dark modes.
It self-elevates to Administrator at startup (UAC) because some actions change
system state. Keep read-only diagnostics usable without elevation where Windows
allows it, and request elevation only when a repair requires it if the startup
flow is later redesigned.

## 2. Product direction and implementation plan

The goal is to make this a practical Windows endpoint support toolkit for
frequent helpdesk work, not a general-purpose automatic repair system. Prefer
small, understandable diagnostics and guided fixes. Report what was checked,
what was found, and what a repair will change. Never claim a repair succeeded
without checking its result.

Build toward this in stages:

1. **Read-only diagnostics and support report.** Add checks for internet and
   gateway reachability, DNS resolution, DHCP, proxy and Wi-Fi state, plus
   device details such as Windows version, hostname, uptime, memory, and free
   storage. Present results in the UI and provide a copyable/exportable report.
2. **Low-risk guided repairs.** Add focused actions for common issues, such as
   restarting a selected Windows service or clearing safe temporary files.
   Show the detected state and proposed action before running it; verify and
   report the outcome.
3. **Disruptive repairs with explicit review.** Keep changes that can interrupt
   connectivity or user work (network resets, adapter/DNS/IP changes, service
   changes with broad impact) behind a confirmation showing the current state
   and planned change. Do not run repairs automatically in the background.

Keep the scope centered on Windows PC support. The app cannot resolve every
support issue: hardware faults, account/permission problems, and cloud-service
incidents may need a technician or other system. Do not add features that imply
the app can diagnose or fix those cases without evidence.

## 3. Build: use the GNU toolchain (MSVC linker is missing)

The dev machine has `rustc`/`cargo` 1.99 but **no `link.exe`**; only MinGW gcc
(`C:\msys64\mingw64\bin`) is installed. The MSVC toolchain therefore cannot link
build scripts/proc-macros. Build/test/lint with the GNU toolchain:

```bash
cargo +stable-x86_64-pc-windows-gnu build
cargo +stable-x86_64-pc-windows-gnu clippy --all-targets -- -D warnings
cargo +stable-x86_64-pc-windows-gnu fmt --check
cargo +stable-x86_64-pc-windows-gnu test
```

The repo pins the GNU toolchain in `rust-toolchain.toml`, so plain `cargo build`
(and rust-analyzer) already use it; the `+stable-x86_64-pc-windows-gnu` prefix is
optional but equivalent. Without that pin, plain `cargo build` fails with
`linker link.exe not found`. The GNU target and toolchain are already installed.
`Cargo.lock` is committed (binary crate).

## 4. Architecture

```
src/
├── main.rs        # elevation check + eframe entrypoint (no logic)
├── app/           # egui App: state, actions, and rendering
│   ├── mod.rs     # state, worker threads, action dispatch (repairs)
│   ├── nav.rs     # sidebar pages: labels + line icons (pure)
│   └── view.rs    # panel and page rendering
├── theme.rs       # Apple HIG visuals (light/dark) + fonts
├── elevate.rs     # IsUserAnAdmin / ShellExecuteW "runas" relaunch
├── monitor.rs     # pure diff/classification of adapter snapshots (unit-tested)
├── msg.rs         # Msg/StatusMsg/MonitorCmd enums + channel helpers
└── win/           # all OS access; nothing here may touch egui
    ├── mod.rs     # to_wide, exec_method (WMI mid-level helper), return_code
    ├── adapters.rs# Win32_NetworkAdapterConfiguration enumeration (serde)
    ├── dns.rs     # flush (runtime dnsapi lookup), resolve, set/reset DNS
    ├── ip.rs      # static IP, DHCP, release/renew, mask parsing
    ├── net.rs     # ICMP ping via IcmpSendEcho
    ├── public_ip.rs # public IP via WinHTTP (HTTPS, read-only)
    ├── reset.rs   # netsh / ipconfig fallbacks
    ├── scan.rs    # parallel ping sweep + GetIpNetTable (ARP)
    └── watch.rs   # live monitor (on by default): netwatcher events + WMI snapshots
```

Long-running actions run on a `std::thread` and stream `Msg::Log` back over an
`mpsc` channel; `DnsApp::poll` drains it each frame. `WMIConnection` is not
`Send`, so each worker creates its own connection inside the thread. Do not
share a connection across threads. `DnsApp::spawn` always sends `Msg::Done` last
(that clears the busy flag).

The Network status page has a **Live status** monitor enabled by default
(`win/watch.rs`). It is read-only: `netwatcher` reports OS interface/IP changes,
a fallback poll (~15 s) covers DNS-only changes, and public IP is rechecked on a
5-minute timer plus a debounced check after changes (max once per minute). The
UI applies monitor observations only while not busy and never lets it start a
repair. Diff/classification logic lives in `monitor.rs` (no OS access).

## 5. Non-obvious Windows/egui gotchas (hard-won)

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

## 6. Safety rules for agents

Network changes can lock a machine out of the network.

1. Never run, or tell the user to run, state-changing operations (static IP, DNS
   change, adapter reset, service restart, file cleanup) without explicit
   approval.
2. Every state-changing action must show the current state, the planned change,
   and ask for confirmation immediately before applying it. Preserve the reset
   confirmation and bring existing unconfirmed network changes into compliance
   when those flows are next modified.
3. Scan only networks the user owns or administers.
4. Never commit real IPs, hostnames, or credentials.

## 7. Definition of done

- `cargo +stable-x86_64-pc-windows-gnu fmt --check`, `clippy --all-targets --
  -D warnings`, and `test` pass.
- No `unwrap()`/`expect()` outside tests; `win/` never imports `egui`; no I/O in
  `theme.rs`/`msg.rs`.
- Manually verified on Windows **as Administrator** for any change touching
  `win/` (do not claim it works otherwise).

## 8. Supplemental Rust skills

The upstream Rust Skills repository is included at `.rust-skills` as a Git
submodule. Consult `.rust-skills/AGENTS.md` and relevant skill files under
`.rust-skills/skills/` for supplemental Rust guidance, especially for ownership,
error handling, concurrency, and unsafe Windows API code. This project's
instructions take precedence where they define stricter requirements or
project-specific behavior; do not replace this file with the upstream guide.

The `clean-code` skill (from `ciembor/agent-rules-books`) is vendored at
`.opencode/skills/clean-code/` and recorded in `skills-lock.json`. OpenCode
advertises it automatically. Apply `clean-code.mini.md` during everyday
implementation and review, and use `clean-code.md` as the deeper reference.
Treat it as guidance, not licence to refactor unrelated code: keep cleanup
proportional and behaviour-preserving, and the rules in this file take
precedence where they are stricter.
