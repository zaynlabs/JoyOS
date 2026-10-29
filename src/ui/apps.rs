use dioxus::prelude::*;

use super::Icon;
use crate::app::Shell;
use crate::t;

/// Apps as a numbered editorial index rather than another grid of icons.
#[component]
pub fn Apps() -> Element {
    let mut shell = use_context::<Shell>();
    let apps = shell.library.read().apps.clone();
    rsx! {
        div { class: "apps-view",
            p { class: "tip", {t!("apps-tip")} }
            div { class: "index",
                for (k, app) in apps.into_iter().enumerate() {
                    {
                        let id = app.id.clone();
                        let launch = app.clone();
                        rsx! {
                            button {
                                key: "{app.id}",
                                class: "entry",
                                "data-nav": "",
                                "data-autofocus": (k == 0).then_some("true"),
                                style: "--tile-accent: {app.accent.css()}",
                                onfocus: move |_| shell.focused.set(Some(id.clone())),
                                onclick: move |_| shell.launch(&launch),
                                span { class: "k", {format!("{:02}", k + 1)} }
                                span { class: "label", "{app.name}" }
                                span { class: "go", Icon { name: "arrow" } }
                            }
                        }
                    }
                }
            }
        }
    }
}
