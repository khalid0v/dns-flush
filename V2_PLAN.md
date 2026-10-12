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
