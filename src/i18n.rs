//! Language support. Every word on screen comes from a Fluent file in `locales/`.
//! Files are compiled into the binary, so a language switch is instant.

use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use unic_langid::LanguageIdentifier;

pub struct Language {
    pub code: &'static str,
    /// The language's name, written in that language.
    pub native: &'static str,
    pub rtl: bool,
    source: &'static str,
}

macro_rules! lang {
    ($code:literal, $native:literal, $rtl:literal) => {
        Language {
            code: $code,
            native: $native,
            rtl: $rtl,
            source: include_str!(concat!("../locales/", $code, "/main.ftl")),
        }
    };
}

pub static LANGUAGES: &[Language] = &[
    lang!("en", "English", false),
    lang!("es", "Español", false),
    lang!("fr", "Français", false),
    lang!("de", "Deutsch", false),
    lang!("it", "Italiano", false),
    lang!("pt-BR", "Português", false),
    lang!("ru", "Русский", false),
    lang!("tr", "Türkçe", false),
    lang!("ar", "العربية", true),
    lang!("hi", "हिन्दी", false),
    lang!("ja", "日本語", false),
    lang!("ko", "한국어", false),
    lang!("zh-Hans", "简体中文", false),
];

static BUNDLES: OnceLock<Vec<FluentBundle<FluentResource>>> = OnceLock::new();
static CURRENT: AtomicUsize = AtomicUsize::new(0);

fn bundles() -> &'static [FluentBundle<FluentResource>] {
    BUNDLES.get_or_init(|| {
        LANGUAGES
            .iter()
            .map(|lang| {
                let id: LanguageIdentifier = lang.code.parse().expect("valid language code");
                let mut bundle = FluentBundle::new_concurrent(vec![id]);
                // Isolation marks only help when right-to-left text mixes with game names.
                bundle.set_use_isolating(lang.rtl);
                let resource = FluentResource::try_new(lang.source.to_owned())
                    .unwrap_or_else(|(res, errs)| {
                        tracing::warn!("{}: {} locale errors", lang.code, errs.len());
                        res
                    });
                bundle.add_resource_overriding(resource);
                bundle
            })
            .collect()
    })
}

pub fn index_of(code: &str) -> Option<usize> {
    LANGUAGES.iter().position(|l| l.code.eq_ignore_ascii_case(code))
}

/// Picks the closest language to the system's, falling back to English.
pub fn detect() -> usize {
    let Some(locale) = sys_locale::get_locale() else { return 0 };
    if let Some(i) = index_of(&locale) {
        return i;
    }
    let primary = locale.split(['-', '_']).next().unwrap_or("en");
    LANGUAGES
        .iter()
        .position(|l| l.code.split('-').next() == Some(primary))
        .unwrap_or(0)
}

pub fn set_language(index: usize) {
    CURRENT.store(index.min(LANGUAGES.len() - 1), Ordering::Relaxed);
}

pub fn current() -> &'static Language {
    &LANGUAGES[CURRENT.load(Ordering::Relaxed)]
}

pub fn current_index() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

pub fn translate(key: &str, args: Option<&FluentArgs>) -> String {
    let all = bundles();
    let current = &all[current_index()];
    for bundle in [current, &all[0]] {
        if let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) {
            let mut errors = vec![];
            return bundle.format_pattern(pattern, args, &mut errors).into_owned();
        }
    }
    key.to_owned()
}

/// One message in a specific language, regardless of the current one.
pub fn translate_in(index: usize, key: &str) -> String {
    let bundle = &bundles()[index.min(LANGUAGES.len() - 1)];
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_else(|| key.to_owned())
}

pub fn args<'a>(pairs: impl IntoIterator<Item = (&'a str, FluentValue<'a>)>) -> FluentArgs<'a> {
    let mut args = FluentArgs::new();
    for (k, v) in pairs {
        args.set(k, v);
    }
    args
}

/// `t!("key")` or `t!("key", name = value, count = 3)`.
#[macro_export]
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::translate($key, None)
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::translate(
            $key,
            Some(&$crate::i18n::args([$((stringify!($name), fluent_bundle::FluentValue::from($value))),+])),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(source: &str) -> Vec<String> {
        let res = FluentResource::try_new(source.to_owned()).expect("locale parses cleanly");
        res.entries()
            .filter_map(|e| match e {
                fluent_syntax::ast::Entry::Message(m) => Some(m.id.name.to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_language_has_every_key() {
        let english = keys(LANGUAGES[0].source);
        assert!(english.len() > 50);
        for lang in &LANGUAGES[1..] {
            let theirs = keys(lang.source);
            let missing: Vec<_> = english.iter().filter(|k| !theirs.contains(k)).collect();
            assert!(missing.is_empty(), "{} is missing {missing:?}", lang.code);
        }
    }

    #[test]
    fn plurals_and_args_format() {
        set_language(0);
        assert_eq!(crate::t!("games-count", count = 1), "1 game");
        assert_eq!(crate::t!("games-count", count = 12), "12 games");
        set_language(index_of("ru").unwrap());
        assert_eq!(crate::t!("games-count", count = 3), "3 игры");
        set_language(0);
    }
}
