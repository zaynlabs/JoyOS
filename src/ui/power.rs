use dioxus::prelude::*;

use super::Icon;
use crate::app::Shell;
use crate::kiosk;
use crate::system::power;
use crate::t;

#[component]
pub fn Power() -> Element {
    let shell = use_context::<Shell>();
    let choices: [(&'static str, String, fn()); 4] = [
        ("moon", t!("power-sleep"), power::sleep),
        ("restart", t!("power-restart"), power::restart),
        ("power", t!("power-off"), power::turn_off),
        ("desktop", t!("power-desktop"), kiosk::show_desktop),
    ];
    let title = t!("power-title");
    rsx! {
        div { class: "layer", "data-layer": "power", role: "dialog", "aria-modal": "true", "aria-label": "{title}",
            div { class: "scrim", onclick: move |_| shell.back() }
            div { class: "power",
                h2 { "{title}" }
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
