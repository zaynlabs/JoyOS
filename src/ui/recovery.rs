//! The way back to Windows, opened by holding LB + RB + View + Start. It matters
//! most when JoyOS is the whole desktop and there's no taskbar to fall back on.

use dioxus::prelude::*;

use super::Icon;
use crate::app::Shell;
use crate::kiosk;
use crate::t;

#[component]
pub fn Recovery() -> Element {
    let shell = use_context::<Shell>();
    let title = t!("recovery-title");
    rsx! {
        div { class: "layer", "data-layer": "recovery", role: "dialog", "aria-modal": "true", "aria-label": "{title}",
            div { class: "scrim", onclick: move |_| shell.back() }
            div { class: "recovery glass",
                h2 { class: "italic", "{title}" }
                p { {t!("recovery-body")} }
                div { class: "choices", "data-row": "recovery",
                    button { class: "row-btn", "data-nav": "", "data-autofocus": "true",
                        onclick: move |_| {
                            shell.close_layers();
                            kiosk::show_desktop();
                        },
                        span { class: "start", Icon { name: "desktop" } {t!("recovery-desktop")} }
                    }
                    button { class: "row-btn", "data-nav": "",
                        onclick: move |_| {
                            if !kiosk::show_undo_script() {
                                shell.toast(t!("kiosk-failed"));
                            }
                        },
                        span { class: "start", Icon { name: "folder" } {t!("recovery-script")} }
                    }
                }
            }
        }
    }
}
