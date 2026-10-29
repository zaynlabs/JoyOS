use dioxus::prelude::*;

use super::Icon;
use crate::app::Shell;
use crate::system::{power, window};
use crate::t;

#[component]
pub fn Power() -> Element {
    let shell = use_context::<Shell>();
    let choices: [(&'static str, String, fn()); 4] = [
        ("moon", t!("power-sleep"), power::sleep),
        ("restart", t!("power-restart"), power::restart),
        ("power", t!("power-off"), power::turn_off),
        ("desktop", t!("power-desktop"), window::minimize_us),
    ];
    rsx! {
        div { class: "layer", "data-layer": "power",
            div { class: "scrim", onclick: move |_| shell.back() }
            div { class: "power",
                h2 { {t!("power-title")} }
                div { class: "choices", "data-row": "power",
                    for (k, (icon, label, action)) in choices.into_iter().enumerate() {
                        button {
                            class: "orb",
                            "data-nav": "",
                            "data-autofocus": (k == 0).then_some("true"),
                            style: "--k: {k}",
                            onclick: move |_| {
                                shell.close_layers();
                                action();
                            },
                            span { class: "disc glass", Icon { name: icon } }
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
