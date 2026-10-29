use std::sync::atomic::Ordering;

use dioxus::prelude::*;

use super::{Icon, sliders::Slider};
use crate::app::{Layer, Shell};
use crate::input::SHARED;
use crate::system::{audio, display, off_thread};
use crate::t;

#[component]
pub fn Quick() -> Element {
    let shell = use_context::<Shell>();
    let mut mouse = use_signal(|| SHARED.mouse_mode.load(Ordering::Relaxed));

    use_hook(move || {
        let (mut volume, mut brightness) = (shell.volume, shell.brightness);
        volume.set(audio::volume());
        spawn(async move {
            brightness.set(off_thread(display::brightness).await.flatten());
        });
    });

    let outside = shell.outside.read().clone();
    let volume = shell.volume.read().unwrap_or(0.0);
    let brightness = *shell.brightness.read();

    rsx! {
        div { class: "layer", "data-layer": "quick",
            div { class: "scrim", onclick: move |_| shell.back() }
            aside { class: "sheet glass lens",
                div { class: "when",
                    div { class: "clock", "data-clock": "" }
                    div { class: "date", "data-date": "" }
                }
                if let Some(o) = outside {
                    div { class: "group",
                        button { class: "row-btn loud", "data-nav": "", "data-autofocus": "true", onclick: move |_| shell.return_outside(),
                            span { class: "start", Icon { name: "return" } {t!("quick-return", name = o.title.clone())} }
                        }
                        button { class: "row-btn danger", "data-nav": "", onclick: move |_| shell.close_outside(),
                            span { class: "start", Icon { name: "close" } {t!("quick-close", name = o.title.clone())} }
                        }
                    }
                }
                div { class: "group",
                    Slider { id: "volume", icon: "sound", label: t!("quick-sound"), value: volume, shown: format!("{}", (volume * 100.0).round()) }
                    if let Some(b) = brightness {
                        Slider { id: "brightness", icon: "sun", label: t!("quick-brightness"), value: b as f32 / 100.0, shown: format!("{b}") }
                    }
                }
                div { class: "group",
                    button {
                        class: "row-btn",
                        "data-nav": "",
                        onclick: move |_| {
                            let on = !mouse();
                            SHARED.mouse_mode.store(on, Ordering::Relaxed);
                            mouse.set(on);
                        },
                        span { class: "start", Icon { name: "mouse" } {t!("quick-mouse")} }
                        span { class: "state", if mouse() { {t!("on")} } else { {t!("off")} } }
                    }
                    button { class: "row-btn", "data-nav": "", onclick: move |_| shell.screenshot(),
                        span { class: "start", Icon { name: "camera" } {t!("quick-screenshot")} }
                    }
                    if shell.outside.read().is_some() {
                        button {
                            class: "row-btn",
                            "data-nav": "",
                            onclick: move |_| {
                                shell.return_outside();
                                shell.show_keyboard(false);
                            },
                            span { class: "start", Icon { name: "keyboard" } {t!("quick-keyboard")} }
                        }
                    }
                }
                div { class: "group",
                    button { class: "row-btn", "data-nav": "", onclick: move |_| shell.open(Layer::Power),
                        span { class: "start", Icon { name: "power" } {t!("quick-power")} }
                    }
                }
            }
        }
    }
}
