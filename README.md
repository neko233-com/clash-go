# clash-go

clash-go is a Windows-first native desktop client for Mihomo, written in Rust and GPUI. It is being built as an independent Clash Verge Rev-compatible project: it reuses Mihomo for proxy protocols and config semantics, while offering a native UI and an agent-friendly CLI in the same executable.

> Feature parity with Clash Verge Rev is the project requirement, not a claim about the current milestone. The current version is an early Windows preview; profile management, system proxy/TUN integration, tray controls, and several advanced UI workflows remain in progress. No performance superiority claim is made until the Windows benchmark suite compares equivalent builds and workloads.

## Windows install

Download `clash-go-windows-x64.zip` from [GitHub Releases](https://github.com/neko233-com/clash-go/releases/latest), or run the installer script from a PowerShell terminal:

```powershell
$script = Join-Path $env:TEMP 'clash-go-install.ps1'
$sources = @('https://raw.githubusercontent.com/neko233-com/clash-go/main/scripts/install.ps1', 'https://gh-proxy.org/https://raw.githubusercontent.com/neko233-com/clash-go/main/scripts/install.ps1')
$downloaded = $false
foreach ($source in $sources) { try { Invoke-WebRequest -UseBasicParsing $source -OutFile $script; $downloaded = $true; break } catch {} }
if (-not $downloaded) { throw 'Could not fetch the installer from GitHub or its configured mirror.' }
Unblock-File $script
& $script -Launch
```

The installer detects reachability and latency for GitHub and mirror routes, downloads the latest Windows x64 release, verifies the SHA-256 digest published by GitHub, installs under `%LOCALAPPDATA%\Programs\clash-go`, and adds that directory to the current user's `PATH`. It does not pipe remote PowerShell text directly into the shell. A release must exist before installation can succeed; tagged `v*` releases are built and published by GitHub Actions.

Options:

```powershell
& $script -InstallDir 'D:\Apps\clash-go' -Route auto -Launch
& $script -Route mirror -MirrorBaseUrl 'https://gh-proxy.org/'
& $script -NoPath
```

`-Route` accepts `auto`, `direct`, or `mirror`. By default the installer tests GitHub direct and the documented `gh-proxy.org` accelerator. Set `CLASH_GO_GITHUB_MIRRORS` to a comma-separated list of mirror prefixes, or pass `-MirrorBaseUrl`; selecting a third-party mirror is an explicit trust choice. Release assets are hash-checked before install. GitHub connectivity and measured route results can also be inspected in Settings or with `clash-go network`.

## Desktop and Mihomo

The desktop app connects to an existing Mihomo external controller. It does not download or bundle a proxy core in this milestone. Defaults to `http://127.0.0.1:9090`:

```powershell
$env:CLASH_GO_CONTROLLER = 'http://127.0.0.1:9090'
$env:CLASH_GO_SECRET = 'your-controller-secret' # optional
clash-go.exe
```

Configure Mihomo's `external-controller` and `secret` to match. Keep an unauthenticated controller bound to localhost. Diagnostic output redacts URL user information, query strings and the controller secret.

## Agent CLI

The installed `clash-go.exe` doubles as a CLI. Use `--json` for one-line machine-readable output, and set the same `CLASH_GO_CONTROLLER` / `CLASH_GO_SECRET` environment variables in the agent process.

```powershell
clash-go.exe --help
clash-go.exe --json diagnose
clash-go.exe --json network
clash-go.exe --json update
clash-go.exe --json config get
clash-go.exe config set mode '"rule"'
clash-go.exe config reload
clash-go.exe --json proxy groups
clash-go.exe proxy select 'Proxy' 'Japan 01'
clash-go.exe --json connections list
clash-go.exe connections close <connection-id> # omit id to close all
clash-go.exe --json inspect rules
clash-go.exe --json inspect proxy-providers
clash-go.exe --json inspect rule-providers
clash-go.exe --json inspect memory
clash-go.exe core restart
clash-go.exe core update-geo
clash-go.exe core clear-dns-cache
```

`config set` accepts a JSON value, patches one running Mihomo config key, and reports core validation failures. `config reload` asks Mihomo to reload its configured file. `diagnose` returns application/core versions, mode, ports, active connection counts and GitHub route reachability; if Mihomo is offline it includes a redacted controller error instead of failing before producing diagnostics. It never serializes the secret. The Mihomo operations map to its documented controller API.

## Build from source on Windows

Install stable Rust with the MSVC target and Visual Studio C++ Build Tools with the Windows SDK. From a Visual Studio Developer PowerShell:

```powershell
cargo fmt --all -- --check
cargo check --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

Create and push a version tag matching `Cargo.toml` to publish a Windows x64 release:

```powershell
git tag v0.2.1
git push origin v0.2.1
```

## Compatibility plan

The parity target is the upstream [Clash Verge Rev](https://github.com/clash-verge-rev/clash-verge-rev) desktop client. Upstream features and implementation will be checked against its current documentation as this matrix advances.

| Area | Status |
| --- | --- |
| Native dashboard, live Mihomo status, policy group node selection | Implemented |
| GitHub version check, direct/mirror reachability detection, hash-verified Windows installer | Implemented in this milestone |
| Agent CLI for diagnostics, config get/set/reload, proxy selection, connection operations, rule/provider inspection | Implemented in this milestone |
| Local profiles, subscriptions, scheduled update, Merge and Script profiles, YAML editor | Not implemented |
| Mihomo core download/channels, config validation/restart/upgrade, Geo data, logs and diagnostics export | Not implemented |
| System proxy, system proxy guard, Windows TUN lifecycle/service/firewall recovery, startup, tray, notifications and shortcuts | Not implemented |
| Proxy/rule/provider editors, health checks, latency tools, connection detail and DNS tools | Not implemented in the UI; some are exposed by the CLI or Mihomo API |
| Themes, CSS injection, localization, accessibility, layout and keyboard shortcuts | Not implemented |
| Local/WebDAV backup and synchronization, import/export | Not implemented |
| Windows feature-by-feature acceptance matrix and equivalent-workload performance/memory benchmark | Not implemented |

We will mark feature parity only after each upstream workflow has an implementation and a Windows acceptance result. Performance work will measure cold start, idle/steady-state memory, CPU, UI responsiveness and controller refresh latency with equivalent Mihomo versions and profiles.

## License

GPL-3.0-only. Third-party components retain their own licenses; see their respective project notices.
