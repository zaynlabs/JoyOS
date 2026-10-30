//! Finds what's installed: games from the stores and ROM folders, apps from the Start Menu.

mod accent;
mod art;
mod epic;
mod gog;
mod roms;
mod startmenu;
mod steam;
mod ubisoft;
mod vdf;
mod xbox;

use std::path::PathBuf;

pub use accent::{Rgb, accent_for};
pub use steam::{cache_dir as steam_cache, running_app as running_steam_game};

use crate::settings::RomFolder;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Steam,
    Epic,
    Gog,
    Ubisoft,
    Xbox,
    Rom,
    App,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Steam => "Steam",
            Source::Epic => "Epic",
            Source::Gog => "GOG",
            Source::Ubisoft => "Ubisoft",
            Source::Xbox => "Xbox",
            Source::Rom => "",
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
    /// URI or file handed to the shell to open it. For ROMs, the ROM file.
    pub launch: String,
    /// The emulator a ROM runs in. `None` for everything else.
    pub emulator: Option<String>,
    /// Extra things to pass when opening it. For ROMs, the emulator's template with `{file}`.
    pub args: Option<String>,
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

pub use art::ArtError;

impl Library {
    /// Slow (touches the disk). Run off the UI thread. Online art comes later, from `with_art`.
    pub fn scan(rom_folders: &[RomFolder]) -> Self {
        let mut games = steam::scan();
        games.extend(epic::scan());
        games.extend(gog::scan());
        games.extend(ubisoft::scan());
        games.extend(xbox::scan());
        games.extend(roms::scan(rom_folders));
        let mut games = merge(games);
        art::fill_cached(&mut games);
        for game in &mut games {
            game.accent = accent_for(game);
        }
        // Stores put shortcuts to their games in the Start menu; those are games, not apps.
        let names: std::collections::HashSet<String> = games.iter().map(|g| plain_name(&g.name)).collect();
        let mut apps: Vec<Title> = startmenu::scan().into_iter().filter(|a| !names.contains(&plain_name(&a.name))).collect();
        for app in &mut apps {
            // The icon comes first: the accent colour is taken from it.
            app.cover = crate::system::icon::png_for(std::path::Path::new(&app.launch)).map(|p| media_url(&p));
            app.accent = accent_for(app);
        }
        Self { games, apps }
    }

    /// Looks up missing covers online, a few at a time. Slow; run off the UI thread.
    pub fn with_art(mut self, art_key: &str) -> (Self, Option<ArtError>) {
        let error = art::fill(&mut self.games, art_key);
        for game in self.games.iter_mut() {
            game.accent = accent_for(game);
        }
        (self, error)
    }

    /// Whether any game still has no cover, so an online look-up could help.
    pub fn missing_art(&self) -> bool {
        self.games.iter().any(|g| g.cover.is_none())
    }
}

/// A name reduced to letters and digits, so "DOOM Eternal" and "Doom: Eternal" match.
fn plain_name(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Sorts by name and keeps one entry per game when two stores have it, preferring the one with art.
fn merge(mut games: Vec<Title>) -> Vec<Title> {
    games.sort_by_key(|g| (plain_name(&g.name), g.cover.is_none(), g.last_played.is_none()));
    games.dedup_by(|later, kept| plain_name(&later.name) == plain_name(&kept.name) && later.source != Source::Rom);
    games.sort_by_key(|g| g.name.to_lowercase());
    games
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

    fn game(id: &str, name: &str, source: Source, cover: Option<&str>) -> Title {
        Title {
            id: id.into(),
            name: name.into(),
            source,
            launch: String::new(),
            emulator: None,
            args: None,
            cover: cover.map(String::from),
            hero: None,
            logo: None,
            folder: None,
            last_played: None,
            accent: Rgb::default(),
        }
    }

    #[test]
    fn the_same_game_from_two_stores_shows_once() {
        let games = merge(vec![
            game("epic:a", "DOOM Eternal", Source::Epic, None),
            game("steam:1", "Doom: Eternal", Source::Steam, Some("x.jpg")),
            game("gog:2", "Celeste", Source::Gog, None),
        ]);
        assert_eq!(games.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(), vec!["gog:2", "steam:1"]);
    }

    #[test]
    fn media_urls_round_trip() {
        let p = PathBuf::from(r"C:\Program Files (x86)\Steam\appcache\librarycache\620\library_600x900.jpg");
        let url = media_url(&p);
        let encoded = url.strip_prefix("http://joy.localhost/file/").unwrap();
        assert!(!encoded.contains(' '));
        assert_eq!(decode_media_path(encoded).unwrap(), p);
    }
}
