//! Sliders take focus like a button; left and right then change the value
//! (nav.js sends a `slide` message instead of moving focus).

use std::time::Duration;

use dioxus::prelude::*;

use super::Icon;
use crate::app::Shell;
use crate::system::{audio, display, off_thread};

#[component]
pub fn Slider(id: &'static str, icon: &'static str, label: String, value: f32, shown: String) -> Element {
    rsx! {
        div {
            class: "slider",
            "data-nav": "",
            "data-slider": id,
            tabindex: "0",
            role: "slider",
            "aria-label": "{label}",
            "aria-valuetext": "{shown}",
            style: "--v: {value.clamp(0.0, 1.0)}",
            Icon { name: icon }
            div { class: "bar", div { class: "fill" } }
            span { class: "val", "{shown}" }
        }
    }
}

pub fn slide(shell: Shell, id: &str, step: i32) {
    match id {
        "volume" => {
            let mut volume = shell.volume;
            let next = (volume().unwrap_or(0.5) + step as f32 * 0.05).clamp(0.0, 1.0);
            audio::set_volume(next);
            volume.set(Some(next));
        }
        "brightness" => {
            let mut brightness = shell.brightness;
            let Some(current) = brightness() else { return };
            let next = (current as i32 + step * 10).clamp(0, 100) as u8;
            brightness.set(Some(next));
            // Each change spawns PowerShell, so only apply once the stick stops.
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(250)).await;
                if brightness() == Some(next) {
                    off_thread(move || display::set_brightness(next)).await;
                }
            });
        }
        "mouse-speed" => shell.change(|s| s.mouse_speed = (s.mouse_speed as i32 + step).clamp(1, 10) as u8),
        _ => {}
    }
}
