use dioxus::prelude::*;

use super::{Icon, Tile};
use crate::app::{Filter, Layer, Shell, now};
use crate::library::{Source, Title};
use crate::t;

const RECENT_DAYS: i64 = 30;

pub fn visible_games(shell: &Shell) -> Vec<Title> {
    let filter = *shell.filter.read();
    let query = shell.search.read().trim().to_lowercase();
    let mut games: Vec<Title> = shell
        .library
        .read()
        .games
        .iter()
        .filter(|g| match filter {
            Filter::All => true,
            Filter::Steam => g.source == Source::Steam,
            Filter::Epic => g.source == Source::Epic,
            Filter::Recent => shell.last_played(g).is_some_and(|t| now() - t < RECENT_DAYS * 86_400),
        })
        .filter(|g| query.is_empty() || g.name.to_lowercase().contains(&query))
        .cloned()
        .collect();
    if filter == Filter::Recent {
        games.sort_by_key(|g| std::cmp::Reverse(shell.last_played(g)));
    }
    games
}

#[component]
pub fn Games() -> Element {
    let mut shell = use_context::<Shell>();
    let games = visible_games(&shell);
    let count = games.len();
    let (has_steam, has_epic) = {
        let lib = shell.library.read();
        (lib.games.iter().any(|g| g.source == Source::Steam), lib.games.iter().any(|g| g.source == Source::Epic))
    };
    let filter = *shell.filter.read();
    let query = shell.search.read().clone();
    let mut filters = vec![(Filter::All, t!("filter-all"))];
    if has_steam {
        filters.push((Filter::Steam, t!("filter-steam")));
    }
    if has_epic {
        filters.push((Filter::Epic, t!("filter-epic")));
    }
    filters.push((Filter::Recent, t!("filter-recent")));

    rsx! {
        div { class: "games",
            header {
                div { class: "big",
                    span { class: "num", "{count}" }
                    span { class: "word", "aria-label": t!("games-count", count = count as i64), {t!("tab-games")} }
                }
                div { class: "filters", "data-row": "filters", "data-keep-top": "",
                    for (f, label) in filters {
                        button {
                            class: "chip",
                            "data-nav": "",
                            "aria-pressed": if f == filter { "true" } else { "false" },
                            onclick: move |_| shell.filter.set(f),
                            "{label}"
                        }
                    }
                    button { class: "chip search-chip", "data-nav": "", onclick: move |_| shell.open(Layer::Keyboard),
                        Icon { name: "search" }
                        if query.is_empty() { {t!("games-search")} } else { "{query}" }
                    }
                }
            }
            div { class: "grid-wrap",
                if games.is_empty() {
                    div { class: "none", {t!("games-none")} }
                } else {
                    div { class: "grid",
                        for (k, game) in games.into_iter().enumerate() {
                            Tile { key: "{game.id}", title: game, k, caption: true, autofocus: k == 0 }
                        }
                    }
                }
            }
        }
    }
}
