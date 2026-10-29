//! Shared building blocks. Screens and layers live in their own modules.

pub mod apps;
pub mod detail;
pub mod games;
pub mod home;
pub mod keyboard;
pub mod power;
pub mod quick;
pub mod settings;
pub mod setup;
pub mod sliders;

use dioxus::prelude::*;

use crate::app::{Layer, Shell, Tab, now};
use crate::input::{Family, Pad};
use crate::library::{Source, Title};
use crate::t;

// ---------- small helpers ----------

pub fn hour() -> u16 {
    unsafe { windows::Win32::System::SystemInformation::GetLocalTime().wHour }
}

pub fn greeting() -> String {
    match hour() {
        5..=11 => t!("greeting-morning"),
        12..=17 => t!("greeting-afternoon"),
        18..=22 => t!("greeting-evening"),
        _ => t!("greeting-night"),
    }
}

pub fn when(ts: Option<i64>) -> String {
    let Some(ts) = ts else { return t!("detail-never") };
    let days = (now() - ts).max(0) / 86_400;
    let when = match days {
        0 => t!("when-today"),
        1 => t!("when-yesterday"),
        n => t!("when-days", n = n),
    };
    t!("detail-last-played", when = when)
}

pub fn initial(name: &str) -> String {
    name.chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().collect()).unwrap_or_default()
}

// ---------- icons ----------

/// Line icons drawn for JoyOS on a 24px grid.
fn icon_path(name: &str) -> &'static str {
    match name {
        "play" => "M8 5.2v13.6L19 12z",
        "search" => "M11 4a7 7 0 1 0 0 14a7 7 0 1 0 0-14zM20 20l-4-4",
        "more" => "M6 12h.01M12 12h.01M18 12h.01",
        "sound" => "M4 9.5h3.5L12 5.5v13l-4.5-4H4zM15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11",
        "sun" => "M12 8.5a3.5 3.5 0 1 0 0 7a3.5 3.5 0 1 0 0-7zM12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4",
        "mouse" => "M12 3a5.5 5.5 0 0 0-5.5 5.5v7a5.5 5.5 0 0 0 11 0v-7A5.5 5.5 0 0 0 12 3zM12 7v3",
        "close" => "M6 6l12 12M18 6L6 18",
        "camera" => "M4 8.5h3l1.5-2.5h7L17 8.5h3v10H4zM12 10.5a3 3 0 1 0 0 6a3 3 0 1 0 0-6z",
        "keyboard" => "M3 6.5h18v11H3zM6.5 10h.01M10 10h.01M13.5 10h.01M17 10h.01M8 14h8",
        "power" => "M12 3v8M7 5.8a7.5 7.5 0 1 0 10 0",
        "moon" => "M19.5 14.5A7.5 7.5 0 0 1 9.5 4.5a7.5 7.5 0 1 0 10 10z",
        "restart" => "M4.5 12a7.5 7.5 0 1 0 2.2-5.3M4.5 4v4.5H9",
        "desktop" => "M3 4.5h18v12H3zM9 20h6M12 16.5V20",
        "arrow" => "M5 12h14M13 6l6 6l-6 6",
        "wifi" => "M2.5 9a14 14 0 0 1 19 0M5.5 12.5a9.5 9.5 0 0 1 13 0M8.7 15.8a5 5 0 0 1 6.6 0M12 19.5h.01",
        "plus" => "M12 5v14M5 12h14",
        "minus" => "M5 12h14",
        "folder" => "M3 6.5h6l2 2h10v10H3z",
        "return" => "M9 14l-5-5l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11",
        "menu" => "M5 8h14M5 12h14M5 16h14",
        "view" => "M4 5h10v10H4zM10 9h10v10H10z",
        _ => "",
    }
}

#[component]
pub fn Icon(name: &'static str) -> Element {
    let fill = if name == "play" { "currentColor" } else { "none" };
    let width = if name == "more" { "3.2" } else { "1.8" };
    rsx! {
        svg {
            view_box: "0 0 24 24",
            fill,
            stroke: "currentColor",
            stroke_width: width,
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: icon_path(name) }
        }
    }
}

// ---------- controller glyphs ----------

#[component]
pub fn Glyph(pad: Pad) -> Element {
    let shell = use_context::<Shell>();
    let family = shell.pad.read().as_ref().map(|(_, f)| *f).unwrap_or_default();
    // PlayStation face buttons are shapes, not letters.
    if family == Family::PlayStation {
        let shape = match pad {
            Pad::A => Some("M6 6l12 12M18 6L6 18"),
            Pad::B => Some("M12 5a7 7 0 1 0 0 14a7 7 0 1 0 0-14z"),
            Pad::X => Some("M6 6h12v12H6z"),
            Pad::Y => Some("M12 5l7.5 13h-15z"),
            _ => None,
        };
        if let Some(d) = shape {
            return rsx! {
                span { class: "glyph",
                    svg { view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2.4", stroke_linejoin: "round", path { d } }
                }
            };
        }
    }
    let label = match (family, pad) {
        (Family::Nintendo, Pad::A) => "B",
        (Family::Nintendo, Pad::B) => "A",
        (Family::Nintendo, Pad::X) => "Y",
        (Family::Nintendo, Pad::Y) => "X",
        (Family::Nintendo, Pad::Lb) => "L",
        (Family::Nintendo, Pad::Rb) => "R",
        (Family::PlayStation, Pad::Lb) => "L1",
        (Family::PlayStation, Pad::Rb) => "R1",
        (_, Pad::A) => "A",
        (_, Pad::B) => "B",
        (_, Pad::X) => "X",
        (_, Pad::Y) => "Y",
        (_, Pad::Lb) => "LB",
        (_, Pad::Rb) => "RB",
        (_, Pad::Lt) => "LT",
        (_, Pad::Rt) => "RT",
        (_, Pad::Start) => "",
        (_, Pad::Select) => "",
    };
    let wide = label.len() > 1;
    rsx! {
        span { class: if wide { "glyph wide" } else { "glyph" },
            match pad {
                Pad::Start => rsx! { Icon { name: "menu" } },
                Pad::Select => rsx! { Icon { name: "view" } },
                _ => rsx! { "{label}" },
            }
        }
    }
}

// ---------- backdrop ----------

#[component]
pub fn Backdrop() -> Element {
    let shell = use_context::<Shell>();
    let mut arts = use_signal(Vec::<(u64, String)>::new);
    let mut generation = use_signal(|| 0u64);

    // Wait for focus to settle before swapping the big image, so flicking
    // through a row doesn't strobe the whole screen.
    use_effect(move || {
        let id = shell.focused.read().clone();
        let gen_now = *generation.peek() + 1;
        generation.set(gen_now);
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(160)).await;
            if *generation.peek() != gen_now {
                return;
            }
            let title = id.and_then(|id| shell.find(&id));
            let accent = title.as_ref().map(|t| t.accent.css()).unwrap_or_else(|| "rgb(226 150 92)".into());
            document::eval(&format!("document.documentElement.style.setProperty('--accent', '{accent}')"));
            let art = title.and_then(|t| t.hero.or(t.cover));
            let mut list = arts.write();
            match art {
                Some(url) if list.last().map(|(_, u)| u) != Some(&url) => {
                    list.push((gen_now, url));
                    let excess = list.len().saturating_sub(2);
                    list.drain(..excess);
                }
                None => list.clear(),
                _ => {}
            }
        });
    });

    let crisp = *shell.tab.read() == Tab::Home && shell.layers.read().is_empty();
    rsx! {
        div { class: "backdrop", "data-mode": if crisp { "crisp" } else { "soft" },
            div { class: "ambient" }
            for (key, url) in arts.read().iter().cloned() {
                img { key: "{key}", class: "art", src: "{url}", alt: "" }
            }
            div { class: "shade" }
        }
    }
}

// ---------- frame ----------

#[component]
pub fn TopBar() -> Element {
    let shell = use_context::<Shell>();
    let current = *shell.tab.read();
    let index = Tab::ALL.iter().position(|&t| t == current).unwrap_or(0);
    let status = shell.status.read().clone();
    rsx! {
        header { class: "topbar",
            div { class: "wordmark", "aria-label": "JoyOS",
                span { class: "joy", "Joy" }
                span { class: "os", "OS" }
            }
            nav { class: "tabs glass lens", style: "--i: {index}; --n: {Tab::ALL.len()}",
                div { class: "lens-dot" }
                for tab in Tab::ALL {
                    button {
                        class: "tab",
                        "aria-current": if tab == current { "true" } else { "false" },
                        onclick: move |_| shell.set_tab(tab),
                        "{tab.label()}"
                    }
                }
            }
            div { class: "status",
                if let Some(name) = &status.wifi {
                    span { title: "{name}", Icon { name: "wifi" } }
                }
                if let Some(pct) = status.battery {
                    Battery { pct, charging: status.charging }
                }
                span { class: "clock", "data-clock": "" }
            }
        }
    }
}

#[component]
fn Battery(pct: u8, charging: bool) -> Element {
    let w = (pct as f32 / 100.0 * 13.0).max(1.5);
    rsx! {
        span { style: "display:flex;align-items:center;gap:6px",
            svg { view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.6",
                rect { x: "2.5", y: "7.5", width: "17", height: "9", rx: "2.5" }
                path { d: "M21.5 10.5v3", stroke_linecap: "round" }
                rect { x: "4.5", y: "9.5", width: "{w}", height: "5", rx: "1", fill: if charging { "#7bd88f" } else { "currentColor" }, stroke: "none" }
            }
            "{pct}%"
        }
    }
}

#[component]
pub fn View() -> Element {
    let shell = use_context::<Shell>();
    let tab = *shell.tab.read();
    rsx! {
        main { class: "view", "data-main": "{tab:?}",
            for t in [tab] {
                div { key: "{t:?}", class: "screen",
                    match t {
                        Tab::Home => rsx! { home::Home {} },
                        Tab::Games => rsx! { games::Games {} },
                        Tab::Apps => rsx! { apps::Apps {} },
                        Tab::Settings => rsx! { settings::SettingsView {} },
                    }
                }
            }
        }
    }
}

#[component]
pub fn HintBar() -> Element {
    let shell = use_context::<Shell>();
    let pad = shell.pad.read().clone();
    let top = shell.top();
    let tab = *shell.tab.read();
    let focused = shell.focused.read().clone();
    let on_title = focused.as_ref().is_some_and(|id| !id.starts_with("app:"));
    let on_tile = top.is_none() && matches!(tab, Tab::Home | Tab::Games | Tab::Apps);
    let select = match (on_tile, on_title) {
        (true, true) => t!("action-play"),
        (true, false) if focused.is_some() => t!("action-open"),
        _ => t!("action-select"),
    };
    rsx! {
        footer { class: "hints",
            div { class: "pad",
                span { class: if pad.is_some() { "dot" } else { "dot off" } }
                match &pad {
                    Some((name, _)) => rsx! { "{name}" },
                    None => rsx! { {t!("pad-none")} },
                }
            }
            div { class: "keys",
                if top.is_none() {
                    span { class: "hint", Glyph { pad: Pad::Lb } Glyph { pad: Pad::Rb } {t!("action-tabs")} }
                }
                if on_tile && on_title && tab != Tab::Apps {
                    span { class: "hint", Glyph { pad: Pad::X } {t!("action-more")} }
                }
                if top.is_none() && tab == Tab::Games {
                    span { class: "hint", Glyph { pad: Pad::Y } {t!("action-search")} }
                }
                span { class: "hint", Glyph { pad: Pad::A } "{select}" }
                if top.is_some() || tab != Tab::Home {
                    span { class: "hint", Glyph { pad: Pad::B } {t!("action-back")} }
                }
                if top != Some(Layer::Quick) {
                    span { class: "hint", Glyph { pad: Pad::Start } {t!("action-menu")} }
                }
            }
        }
    }
}

#[component]
pub fn Toasts() -> Element {
    let shell = use_context::<Shell>();
    rsx! {
        div { class: "toasts", "aria-live": "polite",
            for (id, text) in shell.toasts.read().iter().cloned() {
                div { key: "{id}", class: "toast glass", "{text}" }
            }
        }
    }
}

// ---------- tiles ----------

/// A game cover. A launches, focus updates the backdrop.
#[component]
pub fn Tile(title: Title, k: usize, #[props(default)] caption: bool, #[props(default)] autofocus: bool) -> Element {
    let mut shell = use_context::<Shell>();
    let mut broken = use_signal(|| false);
    let id = title.id.clone();
    let accent = title.accent.css();
    let launch = title.clone();
    let source = title.source.label();
    rsx! {
        div {
            button {
                class: "tile",
                "data-nav": "",
                "data-autofocus": if autofocus { "true" } else { None },
                "aria-label": "{title.name}",
                style: "--tile-accent: {accent}; --k: {k}",
                onfocus: move |_| shell.focused.set(Some(id.clone())),
                onclick: move |_| shell.launch(&launch),
                div { class: "cover-type",
                    span { class: "src", lang: "en", "{source}" }
                    span { class: "name", "{title.name}" }
                }
                if let Some(src) = title.cover.clone() && !broken() {
                    img { src, alt: "", loading: "lazy", decoding: "async", onerror: move |_| broken.set(true) }
                }
            }
            if caption {
                div { class: "caption", "{title.name}" }
            }
        }
    }
}

/// Round chip for an app, used on Home.
#[component]
pub fn AppChip(title: Title, k: usize) -> Element {
    let mut shell = use_context::<Shell>();
    let id = title.id.clone();
    let launch = title.clone();
    rsx! {
        button {
            class: "app-chip glass",
            "data-nav": "",
            style: "--tile-accent: {title.accent.css()}; --k: {k}",
            onfocus: move |_| shell.focused.set(Some(id.clone())),
            onclick: move |_| shell.launch(&launch),
            span { class: "mono", {initial(&title.name)} }
            "{title.name}"
        }
    }
}

pub fn source_label(source: Source) -> String {
    match source {
        Source::Steam => t!("filter-steam"),
        Source::Epic => t!("filter-epic"),
        Source::App => t!("tab-apps"),
    }
}
