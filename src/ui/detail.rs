use dioxus::prelude::*;

use super::{Icon, source_label, when};
use crate::app::Shell;
use crate::library::Source;
use crate::license;
use crate::settings::{GameProfile, PowerMode};
use crate::system::launch;
use crate::t;

#[component]
pub fn Detail(id: String) -> Element {
    let shell = use_context::<Shell>();
    let Some(game) = shell.find(&id) else { return rsx! {} };
    let (pinned, plus, profile) = {
        let s = shell.settings.read();
        (s.pinned.contains(&id), license::has_plus(&s.license), s.profiles.get(&id).copied().unwrap_or_default())
    };
    let art = game.hero.clone().or(game.cover.clone());
    let play = game.clone();
    let pin_id = id.clone();
    let folder = game.folder.clone().filter(|f| f.exists());
    let (pin_icon, pin_label) = if pinned { ("minus", t!("detail-unpin")) } else { ("plus", t!("detail-pin")) };
    // Store names are brands, written the English way; "Classics" and "Apps" are translated words.
    let brand = (!matches!(game.source, Source::Rom | Source::App)).then_some("en");

    rsx! {
        div { class: "layer", "data-layer": "detail", role: "dialog", "aria-modal": "true", "aria-label": "{game.name}", style: "--tile-accent: {game.accent.css()}",
            div { class: "detail",
                if let Some(src) = art {
                    img { class: "art", src, alt: "" }
                }
                div { class: "eyebrow", lang: brand, {source_label(game.source)} }
                if let Some(logo) = game.logo.clone() {
                    img { class: "logo", src: logo, alt: "{game.name}" }
                } else {
                    h1 { "{game.name}" }
                }
                div { class: "when", {when(shell.last_played(&game))} }
                div { class: "actions", "data-row": "actions",
                    button { class: "pill primary", "data-nav": "", "data-autofocus": "true", onclick: move |_| shell.launch(&play),
                        Icon { name: "play" }
                        {t!("action-play")}
                    }
                    button {
                        class: "pill glass",
                        "data-nav": "",
                        onclick: move |_| {
                            let id = pin_id.clone();
                            shell.change(move |s| {
                                if let Some(i) = s.pinned.iter().position(|p| *p == id) {
                                    s.pinned.remove(i);
                                } else {
                                    s.pinned.insert(0, id);
                                }
                            });
                        },
                        Icon { name: pin_icon }
                        "{pin_label}"
                    }
                    if let Some(folder) = folder {
                        button { class: "pill glass", "data-nav": "", onclick: move |_| { launch::show_folder(&folder); },
                            Icon { name: "folder" }
                            {t!("detail-files")}
                        }
                    }
                }
                if game.source != Source::App {
                    if plus {
                        GameSettings { id: id.clone(), profile }
                    } else {
                        div { class: "profile locked", {t!("profile-needs-plus")} }
                    }
                }
            }
        }
    }
}

/// Per-game overrides. Each row cycles through the default and the options.
#[component]
fn GameSettings(id: String, profile: GameProfile) -> Element {
    let shell = use_context::<Shell>();
    let swap = match profile.swap_ab {
        None => t!("profile-default"),
        Some(true) => t!("on"),
        Some(false) => t!("off"),
    };
    let power = match profile.power {
        None => t!("profile-default"),
        Some(PowerMode::Balanced) => t!("power-balanced"),
        Some(PowerMode::Performance) => t!("power-performance"),
        Some(PowerMode::Saver) => t!("power-saver"),
    };
    let swap_id = id.clone();
    let power_id = id.clone();
    rsx! {
        div { class: "profile", "data-row": "profile",
            button { class: "chip", "data-nav": "", onclick: move |_| {
                let id = swap_id.clone();
                shell.change(move |s| {
                    let p = s.profiles.entry(id).or_default();
                    p.swap_ab = match p.swap_ab {
                        None => Some(true),
                        Some(true) => Some(false),
                        Some(false) => None,
                    };
                });
            },
                span { class: "label", {t!("detail-swap")} }
                span { class: "state", "{swap}" }
            }
            button { class: "chip", "data-nav": "", onclick: move |_| {
                let id = power_id.clone();
                shell.change(move |s| {
                    let p = s.profiles.entry(id).or_default();
                    p.power = match p.power {
                        None => Some(PowerMode::Balanced),
                        Some(PowerMode::Balanced) => Some(PowerMode::Performance),
                        Some(PowerMode::Performance) => Some(PowerMode::Saver),
                        Some(PowerMode::Saver) => None,
                    };
                });
            },
                span { class: "label", {t!("detail-power")} }
                span { class: "state", "{power}" }
            }
        }
    }
}
