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

### Tauri builds without MSVC (verified 2026-10-13)

A **Tauri v2** desktop app builds and runs on this machine with the same GNU
toolchain — the MSVC Build Tools install (~2–4 GB, needs admin) is **not**
required, which matters for the planned move from egui to a Tauri + React UI:

- In the Tauri crate's `Cargo.toml`, set
  `[lib] crate-type = ["staticlib", "rlib"]`. Keep `cdylib` **out** of that
  list: including it makes MinGW's `ld` abort with
  `error: export ordinal too large: <n>` because Tauri's DLL exports exceed the
  PE export-ordinal limit.
- `cdylib` is only needed for the mobile (iOS/Android) targets. The Windows
  binary links the `rlib`/`staticlib` directly, so dropping it is safe here.
- The `WebView2Loader` import library is resolved automatically from
  `webview2-com-sys`'s build output, and the WebView2 runtime (154.x) is
  already installed — no extra setup.

Verified with a throwaway `react-ts` Tauri probe: `cargo build` exited 0 and
the built exe opened a WebView2 window.

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

On load the Network status page selects the adapter most likely to be live —
the first with a real (non-APIPA) address and a default gateway — rather than
the first in the WMI list; a manual choice is kept across refreshes.

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

# Windows IT Support Toolkit V2 Plan

## Goal

Evolve the current DNS and network utility into a focused Windows endpoint
support toolkit for frequent helpdesk work. V2 should help a technician identify
common problems, understand what a repair will change, apply approved repairs,
and share a useful support report. It should not imply that every PC or service
issue can be diagnosed or repaired automatically.

Keep the current Rust, `eframe`/`egui`, and Windows-native approach. The app is a
pure Rust desktop utility, so there is no V2 requirement to migrate to Tauri or
add a web frontend.

## V2 user workflow

1. Select **Run checks** to collect read-only system and network diagnostics.
2. Review results grouped by area, with clear status, evidence, and next steps.
3. If a supported repair is appropriate, review the detected state and the
   exact proposed change.
4. Confirm the repair, then see its result and any required follow-up, such as
   reconnecting or restarting Windows.
5. Copy or save a support report, with sensitive details identified and
   removable before sharing.

Checks must never apply repairs automatically. An individual diagnostic may
offer a repair, but the user always initiates and confirms state changes.

## Scope

### Include in V2

- Connectivity checks: adapter link and address state, DHCP configuration,
  gateway reachability, DNS resolution, and a configurable internet reachability
  check.
- Windows network context: active adapter, proxy state, Wi-Fi connection
  details where available, hostname, Windows edition/version, and uptime.
- Basic device health: free disk space and memory information using supported
  Windows APIs or commands, with failures reported as unavailable rather than
  treated as a broken device.
- A support report that combines check results, timestamps, and relevant device
  details. Let the user inspect and redact identifiers and addresses before
  copying or saving it.
- A guided repair flow with a common confirmation and result pattern.
- Preserve and improve the useful existing DNS, DHCP, IP, reset, and subnet
  tools.

### Keep out of V2

- Silent/background repair, unattended or scheduled monitoring, or automatic
  scanning. A user-enabled, read-only, in-app live status view of the local
  machine is allowed.
- Remote device management or scanning networks the operator does not
  administer.
- Claims that the tool can repair hardware faults, identity/account access,
  organization policy, or cloud service incidents.
- A framework rewrite, cloud account, telemetry, or mandatory internet service.

## Delivery stages

### Stage 0 — Safety and groundwork

- Add a common repair review that shows the selected adapter or target, current
  state, planned change, impact, and confirmation control.
- Bring direct DNS changes, DHCP/static IP changes, lease release/renew, and
  AUTO setup into that review flow. Keep the existing Winsock/TCP-IP reset
  confirmation.
- Show that release/renew and network resets may interrupt connectivity.
- Make administrator elevation understandable and avoid requiring it for
  read-only checks where Windows permits them. Recheck permissions when a repair
  is requested.
- Keep settings and reports local; do not collect telemetry.

**Exit criteria:** Every state-changing action has a visible review and
confirmation immediately before execution. Invalid input and WMI/API failures
produce actionable messages and do not report success.

### Stage 1 — Diagnostics and support report (V2 MVP)

- Add a diagnostics overview with explicit **Run checks** and per-check results:
  pass, warning, problem, or unavailable.
- Implement checks as read-only operations. Each result records the check name,
  timestamp, evidence, and concise explanation.
- Prioritize network diagnosis in this order: adapter/link, local address and
  DHCP, gateway, DNS lookup, then internet reachability. This helps show where
  a connection path first fails.
- Add basic device context and proxy/Wi-Fi information, clearly distinguishing
  missing permission or unsupported data from a detected fault.
- Generate a previewable support report. Redact or omit user-identifying
  details by default where practical, and provide copy and save actions.

**Exit criteria:** Running diagnostics changes no system settings; results stay
responsive while checks run; a report can be reviewed before it leaves the app;
each failed check includes its evidence and a useful next step.

### Stage 2 — Guided, targeted repairs

- Offer a repair only when a related diagnostic gives a reason to show it.
- Reuse one repair flow for current state, planned change, impact, confirmation,
  execution, and verified result.
- Add a small first set of repairs: flush DNS cache, renew DHCP lease, and
  restart a specifically selected non-critical service only when its purpose
  and impact are clear.
- For every repair, check the postcondition where possible. Report when a
  restart or reconnect is still required.
- Keep higher-impact operations such as TCP/IP reset and static IP assignment
  available as technician tools with clear warnings and explicit review.

**Exit criteria:** A repair cannot be triggered by merely running diagnostics.
The preview matches the actual action and target. The app reports failures and
partial completion instead of presenting a generic success message.

### Stage 3 — Broaden common endpoint support

After the MVP is used and its highest-value support cases are clear, consider:

- Additional device health checks, such as storage warnings and selected
  Windows service status.
- Safe temporary-file cleanup with an exact preview of target locations and
  estimated space reclaimed. Do not delete user documents, browser profiles,
  or files outside the stated cleanup scope.
- More guided checks for proxy and Wi-Fi issues, with repairs added only when
  their effects can be clearly previewed and verified.
- An expanded support report format if technicians need it for their ticketing
  workflow.

Treat this stage as a prioritized backlog, not a commitment to add every item.

## Implementation shape

- Keep OS access in `src/win/`; it must not depend on egui.
- Add a diagnostics layer that turns Windows observations into typed results
  without rendering UI. Keep check logic separate from presentation.
- Represent repairs as explicit actions with a target, a current-state summary,
  a proposed change, an impact note, and an execution result. The UI owns the
  user's confirmation; the worker performs only the confirmed action.
- Continue running long operations off the UI thread. Return structured
  diagnostic and repair messages to the app, and ensure completion/failure
  always clears the busy state.
- Keep network scans opt-in, bounded, and restricted to a CIDR the operator is
  authorized to inspect. Label discovered/free addresses as observations, not
  guarantees that an address is unused.
- Use focused modules as the diagnostics and repair scope grows; avoid one large
  UI handler accumulating Windows checks, repair policy, and formatting.

## Quality and release gates

- Follow the repository's GNU Rust toolchain and existing format, clippy, and
  test requirements.
- Add unit tests for pure parsing, result classification, report formatting,
  redaction, and repair preview construction. Keep OS-state-changing behavior
  out of unattended tests.
- Manually verify changes that touch Windows APIs on Windows as Administrator;
  never claim this verification without performing it.
- Check the UI at the minimum supported window size, in light and dark themes,
  and while diagnostics or repairs are running.
- Document that network repair results depend on local permissions, policy,
  connectivity, and Windows configuration.

## Suggested first implementation slice

Start with Stage 0's shared confirmation pattern, then build Stage 1's
network-path diagnostics and report. This makes the existing risky changes
reviewable while delivering a useful V2 workflow before adding more repair
commands.
