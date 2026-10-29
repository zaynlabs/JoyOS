//! On-screen keyboard for searching. It follows the current language's layout,
//! with a switch to Latin letters since most game names use them.

use dioxus::prelude::*;

use crate::app::Shell;
use crate::{i18n, t};

use layout::{LATIN, has_own_script, native_rows, script_sample};

/// Key layouts, shared with the keyboard that floats over other apps.
pub mod layout {
    pub const DIGITS: &str = "1234567890";
    pub const LATIN: &[&str] = &["qwertyuiop", "asdfghjkl", "zxcvbnm"];
    /// What addresses, emails and passwords need, in three rows like the letters.
    pub const SYMBOLS: &[&str] = &["@#&_-+=*%", "/\\:;!?'\"", "()[],.~"];

    /// Letter rows for the current language, if it has its own.
    pub fn native_rows(code: &str) -> Option<&'static [&'static str]> {
        Some(match code {
            "fr" => &["azertyuiop", "qsdfghjklm", "wxcvbn"],
            "de" => &["qwertzuiopü", "asdfghjklöä", "yxcvbnmß"],
            "es" => &["qwertyuiop", "asdfghjklñ", "zxcvbnm"],
            "pt-BR" => &["qwertyuiop", "asdfghjklç", "zxcvbnm"],
            "tr" => &["qwertyuıopğü", "asdfghjklşi", "zxcvbnmöç"],
            "ru" => &["йцукенгшщзхъ", "фывапролджэ", "ячсмитьбю"],
            "ar" => &["ضصثقفغعهخحج", "شسيبلاتنمكط", "ئءؤرىةوزظذد"],
            "hi" => &["अआइईउऊएऐओऔ", "कखगघचछजझटठड", "तथदधनपफबभमय", "रलवशसहािीुूेो"],
            _ => return None,
        })
    }

    /// A short sample of a script, shown on the button that switches to it.
    pub fn script_sample(code: &str) -> &'static str {
        match code {
            "ru" => "АБВ",
            "ar" => "أبج",
            "hi" => "अआइ",
            _ => "ABC",
        }
    }

    /// Only Cyrillic, Arabic and Devanagari need a switch to Latin; other layouts already are.
    pub fn has_own_script(code: &str) -> bool {
        matches!(code, "ru" | "ar" | "hi")
    }

    /// The capital a key types with Shift. Rust's uppercasing doesn't know Turkish
    /// has a dotted capital İ, and ß has no single-letter capital on keyboards.
    pub fn upper(ch: char, code: &str) -> String {
        match ch {
            'i' if code == "tr" => "İ".into(),
            'ß' => "ß".into(),
            _ => ch.to_uppercase().collect(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn shift_follows_the_language() {
            assert_eq!(upper('i', "tr"), "İ");
            assert_eq!(upper('ı', "tr"), "I");
            assert_eq!(upper('i', "en"), "I");
            assert_eq!(upper('ß', "de"), "ß");
            assert_eq!(upper('ж', "ru"), "Ж");
        }

        #[test]
        fn symbol_keys_are_unique() {
            let all: Vec<char> = SYMBOLS.iter().flat_map(|r| r.chars()).collect();
            let mut unique = all.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(all.len(), unique.len());
            assert!(all.contains(&'@') && all.contains(&'.') && all.contains(&'/'));
        }
    }
}

pub fn typed(shell: Shell, key: &str) {
    if key == "\u{8}" {
        backspace(shell);
    } else {
        let mut search = shell.search;
        search.write().push_str(key);
    }
}

pub fn backspace(shell: Shell) {
    let mut search = shell.search;
    search.write().pop();
}

#[component]
pub fn Keyboard() -> Element {
    let shell = use_context::<Shell>();
    let mut search = shell.search;
    let code = i18n::current().code;
    let mut latin = use_signal(|| false);
    let rows: &[&str] = if has_own_script(code) && latin() { LATIN } else { native_rows(code).unwrap_or(LATIN) };
    let query = search.read().clone();

    rsx! {
        div { class: "layer", "data-layer": "keyboard",
            div { class: "scrim", onclick: move |_| shell.back() }
            div { class: "keyboard glass",
                div { class: "typed",
                    if query.is_empty() {
                        span { class: "ph", {t!("games-search")} }
                    } else {
                        "{query}"
                    }
                    span { class: "caret" }
                }
                div { class: "keys",
                    div { class: "krow", "data-row": "digits",
                        for ch in layout::DIGITS.chars() {
                            Key { ch }
                        }
                    }
                    for (r, row) in rows.iter().enumerate() {
                        div { key: "{r}-{latin()}", class: "krow", "data-row": "r{r}",
                            for (i, ch) in row.chars().enumerate() {
                                Key { ch, autofocus: r == 0 && i == 0 }
                            }
                        }
                    }
                    div { class: "krow", "data-row": "actions",
                        if has_own_script(code) {
                            button { class: "key wide", "data-nav": "", onclick: move |_| latin.set(!latin()),
                                if latin() { {script_sample(code)} } else { "ABC" }
                            }
                        }
                        button { class: "key space", "data-nav": "", onclick: move |_| search.write().push(' '), {t!("key-space")} }
                        button { class: "key wide", "data-nav": "", "data-sound": "back", onclick: move |_| backspace(shell), {t!("key-delete")} }
                        button { class: "key wide on", "data-nav": "", onclick: move |_| shell.back(), {t!("action-done")} }
                    }
                }
            }
        }
    }
}

#[component]
fn Key(ch: char, #[props(default)] autofocus: bool) -> Element {
    let shell = use_context::<Shell>();
    let mut search = shell.search;
    rsx! {
        button {
            class: "key",
            "data-nav": "",
            "data-sound": "tick",
            "data-autofocus": autofocus.then_some("true"),
            onclick: move |_| search.write().push(ch),
            "{ch}"
        }
    }
}
