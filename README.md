# clash-go

A Windows-first, native Rust + GPUI desktop client for Mihomo. The UI talks to Mihomo's documented controller API; Mihomo remains the proxy engine so existing Clash/Mihomo profiles and protocol support stay compatible.

This repository is an independent implementation. It does not copy Clash Verge Rev source code. The first milestone is a small, native GPUI shell and a live controller dashboard; full Clash Verge Rev feature parity is a project goal and is **not complete yet**.

## Current milestone

- Native GPUI window with Overview, Proxies, Profiles, Rules, Connections, Logs, and Settings navigation.
- Connect to an existing Mihomo controller, show its version, mode, mixed port, policy group count, live connection count, and traffic totals.
- Select an existing node in a Mihomo policy group.
- Keep the UI responsive while controller requests run in the background.
- Windows MSVC build check in CI.

## Windows development

Install the Rust MSVC toolchain and the [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the Windows SDK.

```powershell
$env:CLASH_GO_CONTROLLER = "http://127.0.0.1:9090"
$env:CLASH_GO_SECRET = "your-controller-secret" # optional
cargo run
```

The controller URL defaults to `http://127.0.0.1:9090`. `CLASH_GO_SECRET` is sent as an HTTP Bearer token and is never shown in the UI. Configure Mihomo's `external-controller` and `secret` to match. Avoid exposing an unauthenticated controller outside localhost.

Build for Windows x64:

```powershell
cargo check --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

## Clash Verge Rev compatibility roadmap

The goal is feature parity with [Clash Verge Rev](https://github.com/clash-verge-rev/clash-verge-rev), delivered in reviewable Windows-first milestones.

- [x] Native GPUI shell and Mihomo controller overview
- [x] Read policy groups and switch selected nodes
- [ ] Profile lifecycle: local files, subscriptions, update scheduling, Merge and Script
- [ ] System proxy controls, guard mode, startup behavior, tray menu and notifications
- [ ] Windows TUN lifecycle, permissions, firewall handling and recovery
- [ ] Rule and proxy-provider inspection, editing, health checks and YAML assistance
- [ ] Mihomo core management, stable/alpha channels, restart, config validation and logs
- [ ] Theme colors, tray/proxy icons, CSS injection compatibility where meaningful
- [ ] WebDAV backup and synchronization
- [ ] Import/export, localization, updater, accessibility and diagnostics
- [ ] Windows feature-parity matrix and resource benchmarks against Clash Verge Rev

“Faster” and “lower memory use” are targets, not measured claims. Before making those claims, the project needs repeatable Windows release-build benchmarks that compare cold start, idle/steady-state memory, CPU, and controller refresh latency with the same Mihomo version and profile.

## Performance approach

- Use GPUI for a native GPU-accelerated interface without an embedded browser runtime.
- Reuse Mihomo's core instead of reimplementing proxy protocols and configuration semantics.
- Keep controller I/O off the UI thread and render only the state needed by the current view.
- Publish benchmark environment, exact builds, and measurement method before comparing results.

## License

GPL-3.0-only. Third-party components retain their own licenses; see their respective project notices.
