//! First run: hello → language → controller → finding games → done.

use std::time::Duration;

use dioxus::prelude::*;

use super::settings::Languages;
use crate::app::Shell;
use crate::i18n::{self, LANGUAGES};
use crate::input::Pad;
use crate::t;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Hello,
    Language,
    Controller,
    Finding,
}

#[component]
pub fn Setup() -> Element {
    let shell = use_context::<Shell>();
    let mut step = use_signal(|| Step::Hello);
    let current = step();
    // Setup lives outside the language-keyed shell so its step survives a language
    // change; keying the step on the language redraws its words instead.
    let lang = *shell.lang.read();

    rsx! {
        div { class: "setup", "data-layer": "setup", role: "dialog", "aria-modal": "true",
            div { class: "aura" }
            for s in [current] {
                div { key: "{s as u8}-{lang}", class: "step",
                    match s {
                        Step::Hello => rsx! {
                            Hello {}
                            Next { onclick: move |_| step.set(Step::Language) }
                        },
                        Step::Language => rsx! {
                            h2 { {t!("setup-language")} }
                            Languages { autofocus: true }
                            Next { onclick: move |_| step.set(Step::Controller), autofocus: false }
                        },
                        Step::Controller => rsx! {
                            PadArt {}
                            h2 { {t!("setup-pad-title")} }
                            p {
                                {match shell.pad.read().clone() {
                                    Some((name, _)) => t!("pad-connected", name = name),
                                    None => t!("setup-pad-body"),
                                }}
                            }
                            Next { onclick: move |_| step.set(Step::Finding) }
                        },
                        Step::Finding => rsx! { Finding {} },
                    }
                }
            }
        }
    }
}

#[component]
fn Next(onclick: EventHandler<()>, #[props(default = true)] autofocus: bool) -> Element {
    rsx! {
        button {
            class: "pill primary",
            "data-nav": "",
            "data-autofocus": autofocus.then_some("true"),
            onclick: move |_| onclick.call(()),
            {t!("action-continue")}
        }
    }
}

/// "Hello" in every language JoyOS speaks, one after another.
#[component]
fn Hello() -> Element {
    let mut index = use_signal(|| 0usize);
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(2400)).await;
            index.set((index() + 1) % LANGUAGES.len());
        }
    });
    // A keyed one-item list, so each word is a fresh element and its animation replays.
    rsx! {
        for i in [index()] {
            div { key: "{i}", class: "hello", lang: LANGUAGES[i].code, dir: if LANGUAGES[i].rtl { "rtl" } else { "ltr" },
                {i18n::translate_in(i, "hello")}
            }
        }
    }
}

#[component]
fn Finding() -> Element {
    let shell = use_context::<Shell>();
    let count = shell.library.read().games.len();
    let done = !*shell.scanning.read();
    rsx! {
        div { class: "counter", "{count}" }
        if done {
            h2 { {t!("setup-ready")} }
            p { {t!("games-count", count = count as i64)} }
            button {
                class: "pill primary",
                "data-nav": "",
                "data-autofocus": "true",
                onclick: move |_| shell.change(|s| s.set_up = true),
                {t!("setup-start")}
            }
        } else {
            h2 { {t!("setup-finding")} }
        }
    }
}

/// A controller drawing whose buttons light up as you press them.
#[component]
fn PadArt() -> Element {
    let shell = use_context::<Shell>();
    let mut lit = use_signal(|| None::<Pad>);
    use_effect(move || {
        let pressed = *shell.last_press.read();
        lit.set(pressed);
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(260)).await;
            if *lit.peek() == pressed {
                lit.set(None);
            }
        });
    });
    let on = |p: Pad| if lit() == Some(p) { "lit" } else { "" };
    rsx! {
        svg { class: "pad-art", view_box: "0 0 400 260", fill: "none", stroke: "currentColor", stroke_width: "3", stroke_linejoin: "round",
            path { d: "M118 44h164c52 0 80 30 96 84l18 72c8 32-12 54-38 54c-20 0-32-12-44-32l-16-28H102l-16 28c-12 20-24 32-44 32c-26 0-46-22-38-54l18-72c16-54 44-84 96-84z" }
            circle { cx: "112", cy: "104", r: "24" }
            circle { cx: "244", cy: "160", r: "24" }
            path { d: "M150 150h12v-12h12v12h12v12h-12v12h-12v-12h-12z" }
            circle { class: on(Pad::Select), cx: "178", cy: "96", r: "7" }
            circle { class: on(Pad::Start), cx: "222", cy: "96", r: "7" }
            circle { class: on(Pad::Y), cx: "300", cy: "76", r: "12" }
            circle { class: on(Pad::X), cx: "276", cy: "100", r: "12" }
            circle { class: on(Pad::B), cx: "324", cy: "100", r: "12" }
            circle { class: on(Pad::A), cx: "300", cy: "124", r: "12" }
            path { class: on(Pad::Lb), d: "M80 34c10-10 30-16 50-14" }
            path { class: on(Pad::Rb), d: "M320 34c-10-10-30-16-50-14" }
        }
    }
}
