use dioxus::prelude::*;

use super::{AppChip, Icon, Tile, greeting, source_label, when};
use crate::app::{Layer, Shell};
use crate::library::Source;
use crate::t;

#[component]
pub fn Home() -> Element {
    let shell = use_context::<Shell>();
    let games = shell.jump_back_in();
    let apps: Vec<_> = shell.library.read().apps.iter().take(12).cloned().collect();
    let focused = shell.focused.read().clone();
    let hero = focused.and_then(|id| shell.find(&id)).or_else(|| games.first().cloned());
    let total = games.len();
    let position = hero.as_ref().and_then(|h| games.iter().position(|g| g.id == h.id));
    let empty = games.is_empty() && !*shell.scanning.read();

    rsx! {
        div { class: "home",
            if let Some(hero) = hero {
                for h in [hero] {
                    section { key: "{h.id}", class: "hero",
                        div { class: "greet", {greeting()} }
                        div { class: "meta",
                            span { class: "eyebrow", lang: if h.source == Source::App { None } else { Some("en") }, {source_label(h.source)} }
                            if let Some(i) = position {
                                span { class: "eyebrow count", {format!("{:02} / {:02}", i + 1, total)} }
                            }
                        }
                        if let Some(logo) = h.logo.clone() {
                            img { class: "logo", src: logo, alt: "{h.name}" }
                        } else {
                            h1 { "{h.name}" }
                        }
                        if h.source != Source::App {
                            div { class: "sub", {when(shell.last_played(&h))} }
                        }
                        div { class: "actions",
                            {
                                let play = h.clone();
                                rsx! {
                                    button { class: "pill primary", "data-nav": "", onclick: move |_| shell.launch(&play),
                                        Icon { name: "play" }
                                        if h.source == Source::App { {t!("action-open")} } else { {t!("action-play")} }
                                    }
                                }
                            }
                            if h.source != Source::App {
                                {
                                    let id = h.id.clone();
                                    rsx! {
                                        button { class: "pill glass", "data-nav": "", onclick: move |_| shell.open(Layer::Detail(id.clone())),
                                            Icon { name: "more" }
                                            {t!("action-more")}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if empty {
                section { class: "empty",
                    h1 { class: "italic", {t!("home-empty-title")} }
                    p { {t!("home-empty-body")} }
                }
            } else {
                div {}
            }

            div { class: "rows",
                if !games.is_empty() {
                    div { class: "row-label",
                        span { class: "n", "01" }
                        h2 { {t!("home-jump-back")} }
                    }
                    div { class: "strip", "data-row": "games",
                        for (k, game) in games.into_iter().enumerate() {
                            Tile { key: "{game.id}", title: game, k, autofocus: k == 0 }
                        }
                    }
                }
                if !apps.is_empty() {
                    div { class: "row-label",
                        span { class: "n", if total > 0 { "02" } else { "01" } }
                        h2 { {t!("home-your-apps")} }
                    }
                    div { class: "strip apps", "data-row": "apps",
                        for (k, app) in apps.into_iter().enumerate() {
                            AppChip { key: "{app.id}", title: app, k }
                        }
                    }
                }
            }
        }
    }
}
