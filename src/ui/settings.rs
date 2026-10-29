use dioxus::prelude::*;

use super::{Icon, sliders::Slider};
use crate::app::{Layer, Shell};
use crate::i18n::{self, LANGUAGES};
use crate::settings::{Keyboard, Theme};
use crate::system::startup;
use crate::t;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    Language,
    Look,
    Controller,
    System,
    About,
}

impl Section {
    const ALL: [Section; 5] = [Section::Language, Section::Look, Section::Controller, Section::System, Section::About];

    fn label(self) -> String {
        match self {
            Section::Language => t!("set-language"),
            Section::Look => t!("set-look"),
            Section::Controller => t!("set-controller"),
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
                        Section::System => rsx! { System {} },
                        Section::About => rsx! { p { class: "about", {t!("about-line", version = env!("CARGO_PKG_VERSION"))} } },
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
        button { class: "chip", "data-nav": "", "aria-pressed": if on { "true" } else { "false" }, onclick: move |_| onclick.call(()), "{label}" }
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
            }
        }
        div { class: "option",
            span { class: "name", {t!("set-motion")} }
            div { class: "choices", "data-row": "motion",
                Choice { label: t!("motion-full"), on: !s.reduce_motion, onclick: move |_| shell.change(|s| s.reduce_motion = false) }
                Choice { label: t!("motion-less"), on: s.reduce_motion, onclick: move |_| shell.change(|s| s.reduce_motion = true) }
            }
        }
        Toggle { label: t!("set-sounds"), on: s.sounds, onclick: move |_| shell.change(|s| s.sounds = !s.sounds) }
    }
}

#[component]
fn Controller() -> Element {
    let shell = use_context::<Shell>();
    let s = shell.settings.read().clone();
    let pad = shell.pad.read().clone();
    rsx! {
        div { class: "option",
            span { class: "name",
                match pad {
                    Some((name, _)) => rsx! { {t!("pad-connected", name = name)} },
                    None => rsx! { {t!("pad-none")} },
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
fn System() -> Element {
    let shell = use_context::<Shell>();
    let mut at_startup = use_signal(startup::is_enabled);
    let scanning = *shell.scanning.read();
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
            button { class: "row-btn", style: "width:100%", "data-nav": "", disabled: scanning, onclick: move |_| shell.rescan(true),
                span { class: "start", Icon { name: "search" } {t!("set-rescan")} }
            }
        }
        div { class: "option",
            button { class: "row-btn", style: "width:100%", "data-nav": "", onclick: move |_| shell.open(Layer::Power),
                span { class: "start", Icon { name: "power" } {t!("quick-power")} }
            }
        }
    }
}
