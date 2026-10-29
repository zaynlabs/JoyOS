//! Finds what's installed: games from Steam and Epic, apps from the Start Menu.

mod accent;
mod epic;
mod startmenu;
mod steam;
mod vdf;

use std::path::PathBuf;

pub use accent::{Rgb, accent_for};
pub use steam::running_app as running_steam_game;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Steam,
    Epic,
    App,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Steam => "Steam",
            Source::Epic => "Epic",
            Source::App => "",
        }
    }
}

/// Something JoyOS can open: a game or an app.
#[derive(Clone, Debug, PartialEq)]
pub struct Title {
    /// Stable across runs, e.g. `steam:620`.
    pub id: String,
    pub name: String,
    pub source: Source,
    /// URI or file handed to the shell to open it.
    pub launch: String,
    /// Portrait cover (2:3).
    pub cover: Option<String>,
    /// Wide background art.
    pub hero: Option<String>,
    /// Transparent title logo.
    pub logo: Option<String>,
    pub folder: Option<PathBuf>,
    /// Unix seconds, if the store records it.
    pub last_played: Option<i64>,
    pub accent: Rgb,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Library {
    pub games: Vec<Title>,
    pub apps: Vec<Title>,
}

impl Library {
    /// Slow (touches the disk and decodes cover art). Run off the UI thread.
    pub fn scan() -> Self {
        let mut games = steam::scan();
        games.extend(epic::scan());
        games.sort_by_key(|g| g.name.to_lowercase());
        games.dedup_by(|a, b| a.id == b.id);
        for game in &mut games {
            game.accent = accent_for(game);
        }
        let mut apps = startmenu::scan();
        for app in &mut apps {
            app.accent = accent_for(app);
        }
        Self { games, apps }
    }
}

/// URL the webview uses to load a local file through the `joy` protocol.
pub fn media_url(path: &std::path::Path) -> String {
    let mut out = String::from("http://joy.localhost/file/");
    for b in path.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn decode_media_path(encoded: &str) -> Option<PathBuf> {
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_urls_round_trip() {
        let p = PathBuf::from(r"C:\Program Files (x86)\Steam\appcache\librarycache\620\library_600x900.jpg");
        let url = media_url(&p);
        let encoded = url.strip_prefix("http://joy.localhost/file/").unwrap();
        assert!(!encoded.contains(' '));
        assert_eq!(decode_media_path(encoded).unwrap(), p);
    }
}
