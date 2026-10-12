mod mihomo;

use gpui::{
    App, Application, Bounds, Context, Div, MouseButton, Render, SharedString, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use mihomo::{ControllerClient, EngineSnapshot, ProxyInfo};
use std::hash::{Hash, Hasher};

const BG: u32 = 0x0b0e14;
const SURFACE: u32 = 0x121821;
const SURFACE_RAISED: u32 = 0x18212d;
const BORDER: u32 = 0x26313f;
const MUTED: u32 = 0x8996a8;
const TEXT: u32 = 0xe7edf5;
const ACCENT: u32 = 0x66d9b3;

const NAV_ITEMS: [(&str, &str); 7] = [
    ("Overview", "◈"),
    ("Proxies", "⇄"),
    ("Profiles", "▤"),
    ("Rules", "⌁"),
    ("Connections", "⌘"),
    ("Logs", "≋"),
    ("Settings", "⚙"),
];

struct ClashGoApp {
    controller: Result<ControllerClient, String>,
    snapshot: Option<EngineSnapshot>,
    active_page: String,
    is_loading: bool,
    started: bool,
    last_error: Option<String>,
}

impl ClashGoApp {
    fn new(controller: Result<ControllerClient, String>) -> Self {
        let last_error = controller.as_ref().err().cloned();
        Self {
            controller,
            snapshot: None,
            active_page: "Overview".to_string(),
            is_loading: false,
            started: false,
            last_error,
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }
        let Ok(controller) = self.controller.as_ref() else {
            return;
        };

        self.is_loading = true;
        self.last_error = None;
        let controller = controller.clone();
        cx.notify();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { controller.snapshot() })
                .await;
            let _ = view.update(cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok(snapshot) => {
                        this.snapshot = Some(snapshot);
                        this.last_error = None;
                    }
                    Err(error) => this.last_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_proxy(&mut self, group: String, node: String, cx: &mut Context<Self>) {
        let Ok(controller) = self.controller.as_ref() else {
            return;
        };
        let controller = controller.clone();
        self.is_loading = true;
        cx.notify();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    controller.select_proxy(&group, &node)?;
                    controller.snapshot()
                })
                .await;
            let _ = view.update(cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok(snapshot) => {
                        this.snapshot = Some(snapshot);
                        this.last_error = None;
                    }
                    Err(error) => this.last_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn sidebar(&self, cx: &Context<Self>) -> Div {
        let mut nav = div().flex().flex_col().gap_1().mt_8();
        for (label, icon) in NAV_ITEMS {
            let selected = self.active_page == label;
            let page = label.to_string();
            let item = div()
                .id(label)
                .flex()
                .items_center()
                .gap_3()
                .h(px(42.0))
                .px_3()
                .rounded(px(8.0))
                .text_sm()
                .text_color(rgb(if selected { TEXT } else { MUTED }))
                .bg(rgb(if selected { SURFACE_RAISED } else { BG }))
                .cursor_pointer()
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.active_page = page.clone();
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .w(px(22.0))
                        .text_color(rgb(if selected { ACCENT } else { MUTED }))
                        .child(icon),
                )
                .child(label);
            nav = nav.child(item);
        }

        div()
            .w(px(232.0))
            .h_full()
            .flex()
            .flex_col()
            .p_5()
            .bg(rgb(BG))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("refresh-status")
                            .size(px(38.0))
                            .rounded(px(12.0))
                            .bg(rgb(0x16382f))
                            .text_color(rgb(ACCENT))
                            .text_lg()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("C"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_color(rgb(TEXT))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("clash-go"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child("NATIVE PROXY CLIENT"),
                            ),
                    ),
            )
            .child(nav)
            .child(div().flex_1())
            .child(
                div()
                    .p_3()
                    .rounded(px(10.0))
                    .bg(rgb(SURFACE))
                    .child(div().text_xs().text_color(rgb(MUTED)).child("ENGINE"))
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().size(px(7.0)).rounded_full().bg(rgb(
                                if self.snapshot.is_some() {
                                    ACCENT
                                } else {
                                    0xe3a85d
                                },
                            )))
                            .child(div().text_sm().text_color(rgb(TEXT)).child(
                                if self.snapshot.is_some() {
                                    "Connected"
                                } else {
                                    "Not connected"
                                },
                            )),
                    ),
            )
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        let connected = self.snapshot.is_some();
        let button_label = if self.is_loading {
            "Connecting…"
        } else {
            "Refresh status"
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .pb_6()
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(TEXT))
                            .child(self.active_page.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(page_description(&self.active_page)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(status_pill(
                        connected,
                        if connected {
                            "CORE ONLINE"
                        } else {
                            "CORE OFFLINE"
                        },
                    ))
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded(px(8.0))
                            .bg(rgb(if self.is_loading {
                                SURFACE_RAISED
                            } else {
                                ACCENT
                            }))
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(if self.is_loading { MUTED } else { 0x0b1915 }))
                            .cursor_pointer()
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| this.refresh(cx)),
                            )
                            .child(button_label),
                    ),
            )
    }

    fn overview(&self) -> Div {
        let snapshot = self.snapshot.as_ref();
        let version = snapshot.map(|s| s.version.as_str()).unwrap_or("—");
        let mode = snapshot.map(|s| s.mode.as_str()).unwrap_or("—");
        let groups = snapshot
            .map(|s| s.group_count.to_string())
            .unwrap_or_else(|| "—".to_string());
        let connections = snapshot
            .map(|s| s.connection_count.to_string())
            .unwrap_or_else(|| "—".to_string());
        let mixed_port = snapshot
            .and_then(|s| s.mixed_port)
            .map(|p| p.to_string())
            .unwrap_or_else(|| "not set".to_string());
        let download = snapshot
            .map(|s| format_bytes(s.download_total))
            .unwrap_or_else(|| "—".to_string());
        let upload = snapshot
            .map(|s| format_bytes(s.upload_total))
            .unwrap_or_else(|| "—".to_string());

        div()
            .flex()
            .flex_col()
            .gap_5()
            .mt_6()
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(stat_card("CORE VERSION", version, "Mihomo controller"))
                    .child(stat_card("PROXY MODE", mode, &format!("Mixed port · {mixed_port}")))
                    .child(stat_card("POLICY GROUPS", &groups, "Available for selection"))
                    .child(stat_card("LIVE CONNECTIONS", &connections, "Current sessions")),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(
                        div()
                            .p_5()
                            .rounded(px(12.0))
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .child(section_title("Traffic", "Totals reported by the Mihomo core"))
                            .child(
                                div()
                                    .flex()
                                    .gap_10()
                                    .mt_6()
                                    .child(traffic_total("↓  DOWNLOAD", &download, ACCENT))
                                    .child(traffic_total("↑  UPLOAD", &upload, 0x79aaff)),
                            ),
                    )
                    .child(
                        div()
                            .p_5()
                            .rounded(px(12.0))
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .child(section_title("Local controller", "Connects to your existing Mihomo instance"))
                            .child(div().mt_5().text_color(rgb(TEXT)).text_sm().child(controller_address()))
                            .child(div().mt_2().text_xs().text_color(rgb(MUTED)).child("Set CLASH_GO_CONTROLLER and optional CLASH_GO_SECRET to customize.")),
                    ),
            )
            .child(
                div()
                    .p_5()
                    .rounded(px(12.0))
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .child(section_title("Next milestones", "Windows-first feature parity"))
                    .child(
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .mt_4()
                            .child(milestone("Profile manager", "Subscriptions, local files, merge and script"))
                            .child(milestone("System integration", "Windows proxy guard and TUN lifecycle"))
                            .child(milestone("Advanced tools", "Rules, WebDAV sync, themes and tray controls")),
                    ),
            )
    }

    fn proxies_page(&self, cx: &Context<Self>) -> Div {
        let mut rows = div().flex().flex_col().gap_2().mt_4();
        if let Some(snapshot) = &self.snapshot {
            let mut groups: Vec<_> = snapshot
                .proxies
                .iter()
                .filter(|(_, proxy)| !proxy.all.is_empty())
                .collect();
            groups.sort_by(|left, right| left.0.cmp(right.0));
            for (group_name, proxy) in groups {
                rows = rows.child(proxy_group_row(group_name, proxy, cx.entity().downgrade()));
            }
            if snapshot.group_count == 0 {
                rows = rows.child(empty_state("No policy groups were returned by the core."));
            }
        } else {
            rows = rows.child(empty_state(
                "Connect to Mihomo to load and change proxy groups.",
            ));
        }

        div()
            .flex()
            .flex_col()
            .mt_6()
            .p_5()
            .rounded(px(12.0))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .child(section_title(
                "Proxy groups",
                "Choose an active node in a Mihomo policy group",
            ))
            .child(rows)
    }

    fn placeholder_page(&self) -> Div {
        div()
            .mt_6()
            .p_6()
            .rounded(px(12.0))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .child(div().text_lg().text_color(rgb(TEXT)).child(format!("{} is on the compatibility roadmap", self.active_page)))
            .child(div().mt_2().text_sm().text_color(rgb(MUTED)).child("The project is being built in Windows-first slices. See README.md for the parity checklist."))
    }

    fn body(&self, cx: &mut Context<Self>) -> Div {
        if let Some(error) = &self.last_error {
            let error_text = error.clone();
            let error_banner = div()
                .mb_4()
                .p_3()
                .rounded(px(8.0))
                .bg(rgb(0x39231f))
                .text_sm()
                .text_color(rgb(0xffc3ad))
                .child(format!("Controller unavailable: {error_text}"));
            let body = if self.active_page == "Overview" {
                self.overview()
            } else if self.active_page == "Proxies" {
                self.proxies_page(cx)
            } else {
                self.placeholder_page()
            };
            return div().child(error_banner).child(body);
        }

        if self.active_page == "Overview" {
            self.overview()
        } else if self.active_page == "Proxies" {
            self.proxies_page(cx)
        } else if self.active_page == "Connections" {
            self.connections_page()
        } else {
            self.placeholder_page()
        }
    }

    fn connections_page(&self) -> Div {
        let rows = if let Some(snapshot) = &self.snapshot {
            if snapshot.connection_count == 0 {
                empty_state("There are no active connections right now.")
            } else {
                div().mt_4().text_sm().text_color(rgb(MUTED)).child(format!(
                    "{} active connections · live connection details are in the next UI slice",
                    snapshot.connection_count
                ))
            }
        } else {
            empty_state("Connect to Mihomo to inspect active connections.")
        };
        div()
            .mt_6()
            .p_5()
            .rounded(px(12.0))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .child(section_title(
                "Connections",
                "Inspect current traffic sessions",
            ))
            .child(rows)
    }
}

impl Render for ClashGoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.started {
            self.started = true;
            self.refresh(cx);
        }
        let header = self.header(cx);
        let body = self.body(cx);

        div()
            .flex()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .font_family("Segoe UI")
            .child(self.sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .p_7()
                    .child(header)
                    .child(
                        div()
                            .id("main-content")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(body),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        let controller = ControllerClient::from_environment();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| ClashGoApp::new(controller)),
        )
        .expect("open clash-go window");
        cx.activate(true);
    });
}

fn page_description(page: &str) -> &'static str {
    match page {
        "Overview" => "A compact view of your local proxy engine",
        "Proxies" => "Review groups and change the active route",
        "Profiles" => "Manage subscription and local configurations",
        "Rules" => "Inspect routing rules and rule providers",
        "Connections" => "Inspect active traffic and matched routes",
        "Logs" => "Review Mihomo runtime output",
        "Settings" => "Configure the app and Windows integration",
        _ => "Mihomo desktop client",
    }
}

fn controller_address() -> SharedString {
    std::env::var("CLASH_GO_CONTROLLER")
        .unwrap_or_else(|_| "http://127.0.0.1:9090".to_string())
        .into()
}

fn status_pill(connected: bool, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded(px(99.0))
        .bg(rgb(if connected { 0x16382f } else { 0x30291f }))
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(if connected { ACCENT } else { 0xe3bd75 }))
        .child(label.to_string())
}

fn stat_card(label: &str, value: &str, detail: &str) -> Div {
    div()
        .p_4()
        .rounded(px(12.0))
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .text_xs()
                .text_color(rgb(MUTED))
                .child(label.to_string()),
        )
        .child(
            div()
                .mt_3()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(TEXT))
                .child(value.to_string()),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(MUTED))
                .child(detail.to_string()),
        )
}

fn section_title(title: &str, subtitle: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_base()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(title.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(MUTED))
                .child(subtitle.to_string()),
        )
}

fn traffic_total(label: &str, value: &str, color: u32) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(rgb(color))
                .child(label.to_string()),
        )
        .child(
            div()
                .text_xl()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(value.to_string()),
        )
}

fn milestone(title: &str, detail: &str) -> Div {
    div()
        .p_3()
        .rounded(px(8.0))
        .bg(rgb(SURFACE_RAISED))
        .child(
            div()
                .text_sm()
                .text_color(rgb(TEXT))
                .child(title.to_string()),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(MUTED))
                .child(detail.to_string()),
        )
}

fn empty_state(message: &str) -> Div {
    div()
        .mt_4()
        .p_4()
        .rounded(px(8.0))
        .bg(rgb(SURFACE_RAISED))
        .text_sm()
        .text_color(rgb(MUTED))
        .child(message.to_string())
}

fn proxy_group_row(name: &str, proxy: &ProxyInfo, view: gpui::WeakEntity<ClashGoApp>) -> Div {
    let group = name.to_string();
    let mut children = div()
        .flex()
        .items_center()
        .justify_between()
        .p_4()
        .rounded(px(8.0))
        .bg(rgb(SURFACE_RAISED))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(name.to_string()),
                )
                .child(div().text_xs().text_color(rgb(MUTED)).child(format!(
                    "{} · {} choices",
                    proxy.kind,
                    proxy.all.len()
                ))),
        )
        .child(
            div().text_xs().text_color(rgb(ACCENT)).child(
                proxy
                    .now
                    .clone()
                    .unwrap_or_else(|| "No selection".to_string()),
            ),
        );

    if !proxy.all.is_empty() {
        let mut choices = div().flex().flex_wrap().gap_2().mt_3();
        for node in &proxy.all {
            let current = proxy.now.as_deref() == Some(node.as_str());
            let node_name = node.clone();
            let group_name = group.clone();
            let listener = view.clone();
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            group_name.hash(&mut hasher);
            node_name.hash(&mut hasher);
            let chip = div()
                .id(("proxy-option", hasher.finish()))
                .px_3()
                .py_2()
                .rounded(px(6.0))
                .bg(rgb(if current { 0x16382f } else { BG }))
                .text_xs()
                .text_color(rgb(if current { ACCENT } else { MUTED }))
                .cursor_pointer()
                .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                    let group = group_name.clone();
                    let node = node_name.clone();
                    let _ = listener.update(cx, |this, cx| this.select_proxy(group, node, cx));
                })
                .child(node.clone());
            choices = choices.child(chip);
        }
        children = children.child(choices);
    }

    div().flex().flex_col().gap_1().child(children)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut amount = bytes as f64;
    let mut unit = 0;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    format!("{amount:.1} {}", UNITS[unit])
}
