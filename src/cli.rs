use crate::github::{GithubClient, is_newer_version};
use crate::mihomo::ControllerClient;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "clash-go",
    version,
    about = "Clash Go desktop client and Mihomo diagnostic CLI"
)]
struct Cli {
    #[arg(long, global = true, help = "Write machine-readable JSON to stdout")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Detect direct GitHub and configured mirror reachability/latency.
    Network,
    /// Check the latest published clash-go release.
    Update,
    /// Collect a redacted diagnostic snapshot of this client and Mihomo.
    Diagnose,
    /// Inspect or change Mihomo's live configuration.
    Config(ConfigArgs),
    /// Inspect groups and switch the selected node.
    Proxy(ProxyArgs),
    /// List or close active Mihomo connections.
    Connections(ConnectionArgs),
    /// Read rules and proxy/rule provider metadata.
    Inspect(InspectArgs),
}

#[derive(Args)]
struct ConfigArgs {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    Get,
    /// Patch one running config key with a JSON value, e.g. config set mode '"rule"'.
    Set {
        key: String,
        json_value: String,
    },
    Reload,
}

#[derive(Args)]
struct ProxyArgs {
    #[command(subcommand)]
    action: ProxyAction,
}

#[derive(Subcommand)]
enum ProxyAction {
    Groups,
    Select { group: String, node: String },
}

#[derive(Args)]
struct ConnectionArgs {
    #[command(subcommand)]
    action: ConnectionAction,
}

#[derive(Subcommand)]
enum ConnectionAction {
    List,
    Close { id: Option<String> },
}

#[derive(Args)]
struct InspectArgs {
    #[command(subcommand)]
    action: InspectAction,
}

#[derive(Subcommand)]
enum InspectAction {
    Rules,
    ProxyProviders,
    RuleProviders,
    Memory,
}

#[derive(Serialize)]
struct UpdateStatus {
    current_version: &'static str,
    latest_version: Option<String>,
    update_available: bool,
    release_url: Option<String>,
    route: Option<String>,
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Network => {
            let github = GithubClient::new()?;
            let report = github.network_report();
            emit(&report, cli.json, |report| {
                println!("GitHub routes:");
                for route in &report.routes {
                    println!(
                        "  {:<18} {:<9} {:>5} ms {}",
                        route.name,
                        if route.reachable {
                            "reachable"
                        } else {
                            "offline"
                        },
                        route.latency_ms.unwrap_or_default(),
                        route.detail.as_deref().unwrap_or("")
                    );
                }
                println!("Selected: {}", report.selected.as_deref().unwrap_or("none"));
            });
            if report.selected.is_none() {
                return Err("No usable GitHub route detected".into());
            }
        }
        Command::Update => {
            let github = GithubClient::new()?;
            let network = github.network_report();
            let release = github.latest_release(&network)?;
            let status = UpdateStatus {
                current_version: env!("CARGO_PKG_VERSION"),
                latest_version: release.as_ref().map(|release| release.tag_name.clone()),
                update_available: release.as_ref().is_some_and(|release| {
                    is_newer_version(&release.tag_name, env!("CARGO_PKG_VERSION"))
                }),
                release_url: release.map(|release| release.html_url),
                route: network.selected,
            };
            if status.latest_version.is_none() && !cli.json {
                println!("No public release has been published yet.");
            } else {
                emit(&status, cli.json, |status| {
                    if status.update_available {
                        println!(
                            "Update available: {}",
                            status.latest_version.as_deref().unwrap_or_default()
                        );
                        if let Some(url) = &status.release_url {
                            println!("{url}");
                        }
                    } else {
                        println!("clash-go {} is up to date.", status.current_version);
                    }
                });
            }
        }
        Command::Diagnose => {
            let controller = ControllerClient::from_environment()?;
            let snapshot = controller.snapshot()?;
            let github = GithubClient::new()?;
            let network = github.network_report();
            let report = json!({
                "app": { "name": "clash-go", "version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS, "arch": std::env::consts::ARCH },
                "mihomo": { "version": snapshot.version, "mode": snapshot.mode, "mixed_port": snapshot.mixed_port, "proxy_groups": snapshot.group_count, "connections": snapshot.connection_count, "download_total": snapshot.download_total, "upload_total": snapshot.upload_total },
                "github_network": network,
                "controller": { "endpoint": controller.redacted_endpoint() }
            });
            emit(&report, cli.json, |report| {
                println!(
                    "{}",
                    serde_json::to_string_pretty(report).unwrap_or_default()
                );
            });
        }
        Command::Config(args) => {
            let controller = ControllerClient::from_environment()?;
            let value = match args.action {
                ConfigAction::Get => controller.get_json("/configs")?,
                ConfigAction::Set { key, json_value } => {
                    let value: Value = serde_json::from_str(&json_value)
                        .map_err(|error| format!("Invalid JSON value: {error}"))?;
                    controller.patch_config(&key, value)?;
                    json!({ "ok": true, "updated_key": key })
                }
                ConfigAction::Reload => {
                    controller.reload_config()?;
                    json!({ "ok": true, "reloaded": true })
                }
            };
            emit(&value, cli.json, |value| {
                println!(
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                )
            });
        }
        Command::Proxy(args) => {
            let controller = ControllerClient::from_environment()?;
            let value = match args.action {
                ProxyAction::Groups => controller.proxy_groups()?,
                ProxyAction::Select { group, node } => {
                    controller.select_proxy(&group, &node)?;
                    json!({ "ok": true, "group": group, "node": node })
                }
            };
            emit(&value, cli.json, |value| {
                println!(
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                )
            });
        }
        Command::Connections(args) => {
            let controller = ControllerClient::from_environment()?;
            let value = match args.action {
                ConnectionAction::List => controller.get_json("/connections")?,
                ConnectionAction::Close { id: Some(id) } => {
                    controller.delete(&format!("/connections/{}", urlencoding::encode(&id)))?;
                    json!({ "ok": true, "closed": id })
                }
                ConnectionAction::Close { id: None } => {
                    controller.delete("/connections")?;
                    json!({ "ok": true, "closed": "all" })
                }
            };
            emit(&value, cli.json, |value| {
                println!(
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                )
            });
        }
        Command::Inspect(args) => {
            let controller = ControllerClient::from_environment()?;
            let value = match args.action {
                InspectAction::Rules => controller.get_json("/rules")?,
                InspectAction::ProxyProviders => controller.get_json("/providers/proxies")?,
                InspectAction::RuleProviders => controller.get_json("/providers/rules")?,
                InspectAction::Memory => controller.get_json("/memory")?,
            };
            emit(&value, cli.json, |value| {
                println!(
                    "{}",
                    serde_json::to_string_pretty(value).unwrap_or_default()
                )
            });
        }
    }
    Ok(())
}

fn emit<T: Serialize>(value: &T, as_json: bool, pretty: impl FnOnce(&T)) {
    if as_json {
        println!(
            "{}",
            serde_json::to_string(value).unwrap_or_else(|_| "null".into())
        );
    } else {
        pretty(value);
    }
}
