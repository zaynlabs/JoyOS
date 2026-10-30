use dioxus::prelude::*;

use super::{Icon, sliders::Slider};
use crate::app::{Layer, PinFor, Shell, TextFor, now, plan_name};
use crate::companion;
use crate::household;
use crate::i18n::{self, LANGUAGES};
use crate::input;
use crate::kiosk;
use crate::license::{self, Plan};
use crate::plugins;
use crate::settings::{Keyboard, Member, Scene, Theme};
use crate::system::{launch, startup};
use crate::t;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    Language,
    Look,
    Controller,
    Library,
    Family,
    Connect,
    Plan,
    System,
    About,
}

impl Section {
    const ALL: [Section; 9] = [
        Section::Language,
        Section::Look,
        Section::Controller,
        Section::Library,
        Section::Family,
        Section::Connect,
        Section::Plan,
        Section::System,
        Section::About,
    ];

    fn label(self) -> String {
        match self {
            Section::Language => t!("set-language"),
            Section::Look => t!("set-look"),
            Section::Controller => t!("set-controller"),
            Section::Library => t!("set-library"),
            Section::Family => t!("set-family"),
            Section::Connect => t!("set-connect"),
            Section::Plan => t!("set-plan"),
            Section::System => t!("set-system"),
            Section::About => t!("set-about"),
        }
    }
}

/// tvOS-style: moving onto a section shows it; moving right goes into it.
#[component]
pub fn SettingsView() -> Element {
    let mut section = use_signal(|| Section::Language);
    let current = section();
    rsx! {
        div { class: "settings",
            nav { class: "sections",
                for (i, s) in Section::ALL.into_iter().enumerate() {
                    button {
                        class: "section-btn",
                        "data-nav": "",
                        "aria-current": if s == current { "true" } else { "false" },
                        onfocus: move |_| section.set(s),
                        onclick: move |_| section.set(s),
                        span { class: "n", {format!("{:02}", i + 1)} }
                        {s.label()}
                    }
                }
            }
            for s in [current] {
                section { key: "{s:?}", class: "panel glass",
                    h3 { {s.label()} }
                    match s {
                        Section::Language => rsx! { Languages {} },
                        Section::Look => rsx! { Look {} },
                        Section::Controller => rsx! { Controller {} },
                        Section::Library => rsx! { Library {} },
                        Section::Family => rsx! { Family {} },
                        Section::Connect => rsx! { Connect {} },
                        Section::Plan => rsx! { PlanPanel {} },
                        Section::System => rsx! { System {} },
                        Section::About => rsx! { About {} },
                    }
                }
            }
        }
    }
}

#[component]
pub fn Languages(#[props(default)] autofocus: bool) -> Element {
    let shell = use_context::<Shell>();
    let current = i18n::current_index();
    rsx! {
        div { class: "langs",
            for (i, lang) in LANGUAGES.iter().enumerate() {
                button {
                    class: "lang-btn",
                    "data-nav": "",
                    "data-autofocus": (autofocus && i == current).then_some("true"),
                    lang: lang.code,
                    dir: if lang.rtl { "rtl" } else { "ltr" },
                    "aria-pressed": if i == current { "true" } else { "false" },
                    onclick: move |_| shell.set_language(i),
                    "{lang.native}"
                }
            }
        }
    }
}

#[component]
fn Choice(label: String, on: bool, onclick: EventHandler<()>) -> Element {
    rsx! {
        button { class: "chip", "data-nav": "", role: "radio", "aria-checked": if on { "true" } else { "false" }, "aria-pressed": if on { "true" } else { "false" }, onclick: move |_| onclick.call(()), "{label}" }
    }
}

#[component]
fn Toggle(label: String, on: bool, onclick: EventHandler<()>) -> Element {
    rsx! {
        div { class: "option",
            span { class: "name", "{label}" }
            button { class: "toggle", "data-nav": "", role: "switch", "aria-label": "{label}", "aria-checked": if on { "true" } else { "false" }, onclick: move |_| onclick.call(()) }
        }
    }
}

/// A line that says what a locked feature needs.
#[component]
fn Locked(text: String) -> Element {
    rsx! {
        div { class: "option locked",
            span { class: "name", "{text}" }
        }
    }
}

#[component]
fn Look() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    rsx! {
        div { class: "option",
            span { class: "name", {t!("set-theme")} }
            div { class: "choices", "data-row": "theme",
                Choice { label: t!("theme-dark"), on: s.theme == Theme::Dark, onclick: move |_| shell.change(|s| s.theme = Theme::Dark) }
                Choice { label: t!("theme-light"), on: s.theme == Theme::Light, onclick: move |_| shell.change(|s| s.theme = Theme::Light) }
                Choice { label: t!("theme-auto"), on: s.theme == Theme::Auto, onclick: move |_| shell.change(|s| s.theme = Theme::Auto) }
            }
        }
        div { class: "option",
            span { class: "name", {t!("set-motion")} }
            div { class: "choices", "data-row": "motion",
                Choice { label: t!("motion-full"), on: !s.reduce_motion, onclick: move |_| shell.change(|s| s.reduce_motion = false) }
                Choice { label: t!("motion-less"), on: s.reduce_motion, onclick: move |_| shell.change(|s| s.reduce_motion = true) }
            }
        }
        div { class: "option",
            span { class: "name", {t!("set-scene")} }
            div { class: "choices", "data-row": "scene",
                Choice { label: t!("scene-still"), on: s.scene == Scene::Still, onclick: move |_| shell.change(|s| s.scene = Scene::Still) }
                Choice { label: t!("scene-aurora"), on: s.scene == Scene::Aurora, onclick: move |_| shell.change(|s| s.scene = Scene::Aurora) }
            }
        }
        Toggle { label: t!("set-contrast"), on: s.high_contrast, onclick: move |_| shell.change(|s| s.high_contrast = !s.high_contrast) }
        Toggle { label: t!("set-text-large"), on: s.large_text, onclick: move |_| shell.change(|s| s.large_text = !s.large_text) }
        Toggle { label: t!("set-sounds"), on: s.sounds, onclick: move |_| shell.change(|s| s.sounds = !s.sounds) }
    }
}

#[component]
fn Controller() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let pad = shell.pad.read().clone();
    let battery = *shell.battery.read();
    rsx! {
        div { class: "option",
            span { class: "name",
                match pad {
                    Some((name, _)) => rsx! { {t!("pad-connected", name = name)} },
                    None => rsx! { {t!("pad-none")} },
                }
            }
        }
        if let Some((level, charging)) = battery {
            div { class: "option",
                span { class: "name", {t!("pad-battery", pct = level as i64)} }
                if charging {
                    span { class: "state", {t!("pad-charging")} }
                }
            }
        }
        Toggle { label: t!("set-swap"), on: s.swap_ab, onclick: move |_| shell.change(|s| s.swap_ab = !s.swap_ab) }
        Toggle { label: t!("set-rumble"), on: s.rumble, onclick: move |_| shell.change(|s| s.rumble = !s.rumble) }
        div { class: "option",
            span { class: "name", {t!("set-keyboard")} }
            div { class: "choices", "data-row": "keyboard",
                Choice { label: t!("keyboard-joy"), on: s.keyboard == Keyboard::Joy, onclick: move |_| shell.change(|s| s.keyboard = Keyboard::Joy) }
                Choice { label: t!("keyboard-windows"), on: s.keyboard == Keyboard::Windows, onclick: move |_| shell.change(|s| s.keyboard = Keyboard::Windows) }
            }
        }
        div { class: "option",
            Slider { id: "mouse-speed", icon: "mouse", label: t!("set-speed"), value: s.mouse_speed as f32 / 10.0, shown: format!("{}", s.mouse_speed) }
        }
    }
}

#[component]
fn Library() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let scanning = *shell.scanning.read();
    let art_state = if s.art_key.is_empty() { t!("art-off") } else { t!("art-on") };
    rsx! {
        div { class: "option",
            span { class: "name", {t!("set-art")} }
            button { class: "chip", "data-nav": "", onclick: move |_| shell.ask(TextFor::ArtKey, &s.art_key.clone()),
                "{art_state}"
            }
        }
        for (i, rom) in s.roms.iter().enumerate() {
            div { key: "rom{i}", class: "option",
                span { class: "name", "{rom.path}" }
                span { class: "state", lang: "en", "{rom.emulator} {rom.args}" }
                button { class: "chip", "data-nav": "", onclick: move |_| shell.change(move |s| {
                    if i < s.roms.len() {
                        s.roms.remove(i);
                    }
                }),
                    {t!("action-remove")}
                }
            }
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| shell.ask(TextFor::RomPath, ""),
                span { class: "start", Icon { name: "folder" } {t!("set-rom-add")} }
            }
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", disabled: scanning, onclick: move |_| shell.rescan(true),
                span { class: "start", Icon { name: "search" } {t!("set-rescan")} }
            }
        }
    }
}

/// The next daily limit when the button is pressed: none, then 30, 60, 90, 120 minutes.
fn next_limit(current: Option<u32>) -> Option<u32> {
    match current {
        None => Some(30),
        Some(30) => Some(60),
        Some(60) => Some(90),
        Some(90) => Some(120),
        _ => None,
    }
}

#[component]
fn Family() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let plus = license::has_plus(&s.license);
    let playing = s.playing.clone();

    if !plus {
        return rsx! {
            Locked { text: t!("plan-needs-plus") }
            Stats {}
        };
    }

    rsx! {
        div { class: "option",
            span { class: "name", {t!("family-playing")} }
            div { class: "choices", "data-row": "who",
                Choice { label: t!("family-everyone"), on: playing.is_none(),
                    onclick: move |_| {
                        if shell.everyone_needs_pin() {
                            shell.open(Layer::Pin(PinFor::Everyone));
                        } else {
                            shell.change(|s| s.playing = None);
                        }
                    }
                }
                for m in s.members.iter().cloned() {
                    {
                        let on = playing.as_deref() == Some(m.id.as_str());
                        let name = m.name.clone();
                        rsx! {
                            Choice { key: "{m.id}", label: name, on, onclick: move |_| {
                                if m.pin_hash.is_some() {
                                    shell.open(Layer::Pin(PinFor::Switch(m.id.clone())));
                                } else {
                                    let id = m.id.clone();
                                    shell.change(move |s| s.playing = Some(id));
                                }
                            } }
                        }
                    }
                }
            }
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| shell.ask(TextFor::Member, ""),
                span { class: "start", Icon { name: "plus" } {t!("family-add")} }
            }
        }
        for m in s.members.iter().cloned() {
            MemberRow { key: "{m.id}", member: m }
        }
        Stats {}
    }
}

#[component]
fn MemberRow(member: Member) -> Element {
    let shell = use_context::<Shell>();
    let id = member.id.clone();
    let pin_id = member.id.clone();
    let limit = member.daily_minutes;
    let limit_label = match limit {
        Some(minutes) => t!("family-limit", minutes = minutes as i64),
        None => t!("family-no-limit"),
    };
    let pin_label = if member.pin_hash.is_some() { t!("family-change-pin") } else { t!("family-set-pin") };
    rsx! {
        div { class: "option",
            span { class: "name", "{member.name}" }
            button { class: "chip", "data-nav": "", onclick: move |_| shell.change({
                let id = id.clone();
                move |s| {
                    if let Some(m) = s.members.iter_mut().find(|m| m.id == id) {
                        m.daily_minutes = next_limit(m.daily_minutes);
                    }
                }
            }), "{limit_label}" }
            button { class: "chip", "data-nav": "", onclick: move |_| shell.open(Layer::Pin(PinFor::Edit(pin_id.clone()))), "{pin_label}" }
            button { class: "chip", "data-nav": "", onclick: move |_| {
                let id = member.id.clone();
                shell.change(move |s| {
                    s.members.retain(|m| m.id != id);
                    if s.playing.as_deref() == Some(id.as_str()) {
                        s.playing = None;
                    }
                });
            },
                Icon { name: "close" }
                {t!("action-remove")}
            }
        }
    }
}

/// Play time and the most played games, from the sessions JoyOS has recorded.
#[component]
fn Stats() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let who = s.playing.clone();
    let who = who.as_deref();
    let week = household::minutes_since(&s.sessions, who, now() - 7 * 86_400);
    let year = household::minutes_since(&s.sessions, who, now() - 365 * 86_400);
    let top = household::top_games(&s.sessions, who, now() - 365 * 86_400, 3);
    let summary = t!("stats-summary", week = i18n::decimal(week as f64 / 60.0, 1), year = i18n::decimal(year as f64 / 60.0, 0));
    rsx! {
        // Focusable, so a controller can scroll it into view and a screen reader reads it out.
        div { class: "stats glass", tabindex: "0", "data-nav": "", "aria-label": "{summary}",
            div { class: "stat",
                span { class: "num", {i18n::decimal(week as f64 / 60.0, 1)} }
                span { class: "word", {t!("stats-week")} }
            }
            div { class: "stat",
                span { class: "num", {i18n::decimal(year as f64 / 60.0, 0)} }
                span { class: "word", {t!("stats-year")} }
            }
            if !top.is_empty() {
                ol { class: "top",
                    for (game, minutes) in top {
                        li { key: "{game}",
                            span { class: "name", {shell.find(&game).map(|t| t.name).unwrap_or(game.clone())} }
                            span { class: "state", {t!("stats-minutes", minutes = minutes as i64)} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Connect() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let plus = license::has_plus(&s.license);
    let address = companion::lan_address().map(|ip| format!("http://{ip}:{}", companion::PORT));
    // Read so a new code shows as soon as a phone pairs.
    let _ = shell.paired.read();
    let pin = companion::pin();
    rsx! {
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| { launch::open("ms-settings:bluetooth"); },
                span { class: "start", Icon { name: "desktop" } {t!("conn-bluetooth")} }
            }
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| { launch::open("ms-settings:network-wifi"); },
                span { class: "start", Icon { name: "wifi" } {t!("conn-wifi")} }
            }
        }
        Toggle {
            label: t!("set-phone"),
            on: s.phone_remote,
            onclick: move |_| {
                let on = !s.phone_remote;
                if on && let Some(tx) = input::sender() {
                    companion::start(tx);
                } else if !on {
                    companion::stop();
                }
                shell.change(|s| s.phone_remote = on);
            },
        }
        if s.phone_remote {
            if let (Some(address), Some(pin)) = (address, pin) {
                div { class: "option",
                    span { class: "name", {t!("phone-open", address = address)} }
                }
                div { class: "option",
                    span { class: "name", {t!("phone-pin", pin = pin)} }
                }
                div { class: "option",
                    button { class: "row-btn", style: "width:100%", "data-nav": "",
                        onclick: move |_| {
                            companion::forget_phones();
                            let mut paired = shell.paired;
                            paired += 1;
                            shell.toast(t!("phone-forgot"));
                        },
                        span { class: "start", Icon { name: "close" } {t!("phone-forget")} }
                    }
                }
            }
        }
        if plus {
            Toggle { label: t!("set-plugins"), on: s.plugins, onclick: move |_| shell.change(|s| s.plugins = !s.plugins) }
            div { class: "option",
                button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| {
                    if let Some(dir) = plugins::folder() {
                        let _ = std::fs::create_dir_all(&dir);
                        launch::show_folder(&dir);
                    }
                },
                    span { class: "start", Icon { name: "folder" } {t!("plugins-open")} }
                }
            }
        } else {
            Locked { text: t!("plan-needs-plus") }
        }
    }
}

#[component]
fn PlanPanel() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let plan = license::plan(&s.license);
    let plus = plan >= Plan::Plus;
    let update = shell.update.read().clone();
    rsx! {
        div { class: "option",
            span { class: "name", {t!("plan-current", plan = plan_name(plan))} }
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| shell.ask(TextFor::License, ""),
                span { class: "start", Icon { name: "keyboard" } {t!("plan-enter-key")} }
            }
        }
        if plan == Plan::Business {
            div { class: "option",
                button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| {
                    match kiosk::write_scripts() {
                        Ok(dir) => {
                            launch::show_folder(&dir);
                            shell.toast(t!("kiosk-made"));
                        }
                        Err(_) => shell.toast(t!("kiosk-failed")),
                    }
                },
                    span { class: "start", Icon { name: "desktop" } {t!("kiosk-make")} }
                }
            }
        } else {
            Locked { text: t!("plan-needs-business") }
        }
        Toggle { label: t!("set-crash"), on: s.crash_reports, onclick: move |_| shell.change(|s| s.crash_reports = !s.crash_reports) }
        div { class: "option",
            span { class: "name", {t!("set-crash-note")} }
        }
        if plus {
            div { class: "option",
                button { class: "chip", "data-nav": "", onclick: move |_| shell.ask(TextFor::SyncFolder, &s.sync_folder.clone().unwrap_or_default()),
                    {t!("sync-folder")}
                }
                if s.sync_folder.is_some() {
                    button { class: "chip", "data-nav": "", onclick: move |_| shell.sync_now(), {t!("sync-now")} }
                }
            }
        } else {
            Locked { text: t!("plan-needs-plus") }
        }
        Toggle { label: t!("set-test-versions"), on: s.test_versions, onclick: move |_| shell.change(|s| s.test_versions = !s.test_versions) }
        div { class: "option",
            button { class: "chip", "data-nav": "", onclick: move |_| shell.ask(TextFor::UpdateUrl, &s.update_url.clone()), {t!("update-address")} }
            if let Some(u) = update {
                button { class: "chip", "data-nav": "", onclick: move |_| { launch::open(&u.url); }, {t!("update-get", version = u.version.clone())} }
            }
        }
    }
}

#[component]
fn System() -> Element {
    let shell = use_context::<Shell>();
    let mut at_startup = use_signal(startup::is_enabled);
    rsx! {
        Toggle {
            label: t!("set-startup"),
            on: at_startup(),
            onclick: move |_| {
                let on = !at_startup();
                startup::set_enabled(on);
                at_startup.set(on);
            },
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| shell.open(Layer::Power),
                span { class: "start", Icon { name: "power" } {t!("quick-power")} }
            }
        }
    }
}

#[component]
fn About() -> Element {
    let shell = use_context::<Shell>();
    let plan = license::plan(&shell.settings.read().license);
    rsx! {
        p { class: "about", {t!("about-line", version = env!("CARGO_PKG_VERSION"))} }
        p { class: "about", {t!("plan-current", plan = plan_name(plan))} }
    }
}
