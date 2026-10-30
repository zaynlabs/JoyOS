//! The Watch tab: streaming sites and video, opened full screen. Four are built in;
//! the rest are sites the user adds.

use dioxus::prelude::*;

use super::Icon;
use crate::app::{Shell, TextFor};
use crate::settings::WebTile;
use crate::t;

const BUILT_IN: &[(&str, &str)] = &[
    ("Netflix", "https://www.netflix.com"),
    ("YouTube", "https://www.youtube.com/tv"),
    ("Twitch", "https://www.twitch.tv"),
    ("Spotify", "https://open.spotify.com"),
];

#[component]
pub fn Watch() -> Element {
    let shell = use_context::<Shell>();
    let mine = shell.settings.read().web_tiles.clone();

    rsx! {
        div { class: "watch",
            header { class: "big",
                span { class: "num", "{BUILT_IN.len() + mine.len()}" }
                span { class: "word", {t!("tab-watch")} }
            }
            p { class: "lede", {t!("watch-lede")} }
            div { class: "strip web", "data-row": "web",
                for (i, (name, url)) in BUILT_IN.iter().enumerate() {
                    {
                        let tile = WebTile { name: name.to_string(), url: url.to_string() };
                        rsx! {
                            button { key: "{url}", class: "web-tile glass", "data-nav": "", "data-id": "{url}", "data-autofocus": (i == 0).then_some("true"),
                                onclick: move |_| shell.launch_web(&tile),
                                span { class: "name", "{name}" }
                                Icon { name: "play" }
                            }
                        }
                    }
                }
                for (i, tile) in mine.iter().enumerate() {
                    {
                        let launch = tile.clone();
                        rsx! {
                            button { key: "m{i}-{tile.url}", class: "web-tile glass", "data-nav": "", "data-id": "{tile.url}",
                                onclick: move |_| shell.launch_web(&launch),
                                span { class: "name", "{tile.name}" }
                                Icon { name: "play" }
                            }
                        }
                    }
                }
                button { class: "web-tile add glass", "data-nav": "", onclick: move |_| shell.ask(TextFor::WebName, ""),
                    Icon { name: "plus" }
                    span { class: "name", {t!("watch-add")} }
                }
            }
            if !mine.is_empty() {
                div { class: "manage",
                    for (i, tile) in mine.iter().enumerate() {
                        div { key: "r{i}-{tile.url}", class: "option",
                            span { class: "name", "{tile.name}" }
                            button { class: "chip", "data-nav": "", onclick: move |_| shell.change(move |s| {
                                if i < s.web_tiles.len() {
                                    s.web_tiles.remove(i);
                                }
                            }),
                                Icon { name: "close" }
                                {t!("action-remove")}
                            }
                        }
                    }
                }
            }
        }
    }
}
