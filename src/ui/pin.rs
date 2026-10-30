//! A PIN pad, for choosing who's playing and for setting a PIN. Six digits at most.

use dioxus::prelude::*;

use crate::app::{PinFor, Shell};
use crate::t;

const LENGTH: usize = 6;

#[component]
pub fn Pin(target: PinFor) -> Element {
    let shell = use_context::<Shell>();
    let mut pin = use_signal(String::new);
    let entered = pin.read().clone();
    let title = match target {
        PinFor::Switch(_) => t!("pin-ask"),
        PinFor::Edit(_) => t!("pin-set"),
        PinFor::Everyone | PinFor::Recovery => t!("pin-grown-up"),
    };
    let confirm_target = target.clone();
    let typed_label = t!("pin-typed", count = entered.len() as i64);

    rsx! {
        div { class: "layer", "data-layer": "pin", role: "dialog", "aria-modal": "true", "aria-label": "{title}",
            div { class: "scrim", onclick: move |_| shell.back() }
            div { class: "pin glass",
                h2 { class: "italic", "{title}" }
                div { class: "dots", role: "status", "aria-live": "polite", "aria-label": "{typed_label}",
                    for i in 0..LENGTH {
                        span { key: "{i}", class: if i < entered.len() { "dot on" } else { "dot" } }
                    }
                }
                div { class: "pad-keys", "data-row": "pin",
                    for digit in ["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
                        button { key: "{digit}", class: "key", "data-nav": "", "data-autofocus": (digit == "1").then_some("true"),
                            onclick: move |_| {
                                let len = pin.read().len();
                                if len < LENGTH {
                                    pin.write().push_str(digit);
                                }
                            },
                            "{digit}"
                        }
                    }
                    button { class: "key", "data-nav": "", "aria-label": t!("pin-clear"),
                        onclick: move |_| pin.set(String::new()),
                        {t!("pin-clear")}
                    }
                    button { class: "key", "data-nav": "",
                        onclick: move |_| {
                            let len = pin.read().len();
                            if len < LENGTH {
                                pin.write().push('0');
                            }
                        },
                        "0"
                    }
                    button { class: "key wide on", "data-nav": "",
                        onclick: move |_| {
                            let typed = pin.read().clone();
                            // A right PIN closes this pad; a wrong one clears it for another try.
                            if !shell.enter_pin(confirm_target.clone(), &typed) {
                                pin.set(String::new());
                            }
                        },
                        {t!("action-done")}
                    }
                }
            }
        }
    }
}
