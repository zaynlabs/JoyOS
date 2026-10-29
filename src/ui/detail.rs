use dioxus::prelude::*;

use super::{Icon, source_label, when};
use crate::app::Shell;
use crate::system::launch;
use crate::t;

#[component]
pub fn Detail(id: String) -> Element {
    let shell = use_context::<Shell>();
    let Some(game) = shell.find(&id) else { return rsx! {} };
    let pinned = shell.settings.read().pinned.contains(&id);
    let art = game.hero.clone().or(game.cover.clone());
    let play = game.clone();
    let pin_id = id.clone();
    let folder = game.folder.clone().filter(|f| f.exists());
    let (pin_icon, pin_label) = if pinned { ("minus", t!("detail-unpin")) } else { ("plus", t!("detail-pin")) };

    rsx! {
        div { class: "layer", "data-layer": "detail", style: "--tile-accent: {game.accent.css()}",
            div { class: "detail",
                if let Some(src) = art {
                    img { class: "art", src, alt: "" }
                }
                div { class: "eyebrow", lang: "en", {source_label(game.source)} }
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
            }
        }
    }
}
