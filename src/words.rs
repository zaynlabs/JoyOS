//! Word suggestions for the keyboards: words you've typed before, most recent first.

use crate::settings::Settings;

/// Enough history to be useful without growing the settings file forever.
const MAX_WORDS: usize = 400;

/// Remembers every word in `text` that's at least two letters long.
pub fn learn(words: &mut Vec<String>, text: &str) {
    for word in text.split_whitespace().filter(|w| w.chars().count() >= 2) {
        words.retain(|w| !w.eq_ignore_ascii_case(word));
        words.insert(0, word.to_owned());
    }
    words.truncate(MAX_WORDS);
}

/// The word being typed: whatever follows the last space.
pub fn fragment(text: &str) -> &str {
    text.rsplit(char::is_whitespace).next().unwrap_or("")
}

/// Up to three remembered words that start with `fragment`.
pub fn suggest(words: &[String], fragment: &str) -> Vec<String> {
    if fragment.is_empty() {
        return vec![];
    }
    let typed = fragment.to_lowercase();
    words
        .iter()
        .filter(|w| {
            let w = w.to_lowercase();
            w.starts_with(&typed) && w != typed
        })
        .take(3)
        .cloned()
        .collect()
}

pub fn remember(settings: &mut Settings, text: &str) {
    learn(&mut settings.words, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_words_come_first_and_dedupe() {
        let mut words = vec![];
        learn(&mut words, "zelda mario");
        learn(&mut words, "Mario a");
        assert_eq!(words, vec!["Mario".to_string(), "zelda".to_string()]);
    }

    #[test]
    fn suggests_prefix_matches_only() {
        let words: Vec<String> = ["Zelda", "Zombies", "Mario", "zero"].iter().map(|s| s.to_string()).collect();
        assert_eq!(suggest(&words, "ze"), vec!["Zelda".to_string(), "zero".to_string()]);
        assert!(suggest(&words, "").is_empty());
        assert!(suggest(&words, "zelda").is_empty(), "a complete word needs no suggestion");
    }

    #[test]
    fn fragment_is_the_last_word() {
        assert_eq!(fragment("super mar"), "mar");
        assert_eq!(fragment("super "), "");
    }
}
