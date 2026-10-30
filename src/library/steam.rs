use std::path::{Path, PathBuf};

use super::{Rgb, Source, Title, media_url, vdf};

/// Tools and runtimes Steam installs alongside games.
const NOT_GAMES: &[&str] = &[
    "228980", "1070560", "1391110", "1628350", "1493710", "2180100", // runtimes
    "250820", "1007", "211", "215", "218", "243730", "243750", // SteamVR, SDKs
];
/// Names that give away a tool rather than a game.
const NOT_GAME_WORDS: &[&str] = &["Redistributable", "Dedicated Server", " SDK", "Proton", "Steam Linux Runtime"];

/// `StateFlags` bits from the manifest.
const FULLY_INSTALLED: u32 = 4;
const UNINSTALLING: u32 = 2048;

fn steam_root() -> Option<PathBuf> {
    crate::system::registry::read_string(r"Software\Valve\Steam", "SteamPath")
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .or_else(|| {
            let fallback = PathBuf::from(r"C:\Program Files (x86)\Steam");
            fallback.exists().then_some(fallback)
        })
}

/// Where Steam keeps its cover art. The `joy` protocol may serve files from here.
pub fn cache_dir() -> Option<PathBuf> {
    steam_root().map(|r| r.join("appcache/librarycache"))
}

pub fn scan() -> Vec<Title> {
    let Some(root) = steam_root() else { return vec![] };
    let mut libraries = vec![root.clone()];
    if let Ok(text) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
        let parsed = vdf::parse(&text);
        if let Some(folders) = parsed.obj("libraryfolders") {
            for value in folders.0.values() {
                if let vdf::Value::Obj(lib) = value
                    && let Some(path) = lib.str("path")
                {
                    let path = PathBuf::from(path);
                    if !libraries.iter().any(|l| same_path(l, &path)) {
                        libraries.push(path);
                    }
                }
            }
        }
    }

    let cache = root.join("appcache/librarycache");
    let mut games = vec![];
    for lib in libraries {
        let Ok(entries) = std::fs::read_dir(lib.join("steamapps")) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
            if let Some(game) = from_manifest(&text, &lib, &cache) {
                games.push(game);
            }
        }
    }
    games
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().replace('/', "\\").eq_ignore_ascii_case(&b.to_string_lossy().replace('/', "\\"))
}

fn from_manifest(text: &str, lib: &Path, cache: &Path) -> Option<Title> {
    let root = vdf::parse(text);
    let app = root.obj("AppState")?;
    let id = app.str("appid")?.to_owned();
    let name = app.str("name")?.to_owned();
    // Missing flags mean an old manifest; those were installed.
    let flags: u32 = app.str("StateFlags").and_then(|s| s.parse().ok()).unwrap_or(FULLY_INSTALLED);
    if NOT_GAMES.contains(&id.as_str())
        || NOT_GAME_WORDS.iter().any(|w| name.contains(w))
        || flags & FULLY_INSTALLED == 0
        || flags & UNINSTALLING != 0
    {
        return None;
    }
    // Only covers on disk: newer games keep their art at hashed paths, so a guessed
    // download address often fails. Missing covers are filled in by `art`.
    let art = |file: &str| find_art(cache, &id, file);
    Some(Title {
        launch: format!("steam://rungameid/{id}"),
        cover: art("library_600x900.jpg"),
        hero: art("library_hero.jpg"),
        logo: art("logo.png"),
        folder: app.str("installdir").map(|d| lib.join("steamapps/common").join(d)),
        last_played: app.str("LastPlayed").and_then(|s| s.parse().ok()).filter(|&t: &i64| t > 0),
        accent: Rgb::default(),
        id: format!("steam:{id}"),
        name,
        source: Source::Steam,
        emulator: None,
        args: None,
    })
}

/// Steam has used three cache layouts over the years; check them all.
fn find_art(cache: &Path, id: &str, file: &str) -> Option<String> {
    let flat = cache.join(format!("{id}_{file}"));
    if flat.exists() {
        return Some(media_url(&flat));
    }
    let dir = cache.join(id);
    let direct = dir.join(file);
    if direct.exists() {
        return Some(media_url(&direct));
    }
    std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path().join(file))
        .find(|p| p.exists())
        .map(|p| media_url(&p))
}

/// Whether Steam says a game is running right now.
pub fn running_app() -> Option<String> {
    let id = crate::system::registry::read_dword(r"Software\Valve\Steam", "RunningAppID")?;
    (id != 0).then(|| format!("steam:{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_becomes_a_game() {
        let text = r#""AppState" { "appid" "620" "name" "Portal 2" "installdir" "Portal 2" "LastPlayed" "1700000000" }"#;
        let game = from_manifest(text, Path::new(r"D:\Steam"), Path::new(r"Z:\nowhere")).unwrap();
        assert_eq!(game.id, "steam:620");
        assert_eq!(game.launch, "steam://rungameid/620");
        assert_eq!(game.last_played, Some(1_700_000_000));
        assert_eq!(game.cover, None, "no art on disk; the art step fills it in");
    }

    #[test]
    fn skips_runtimes_and_tools() {
        let text = r#""AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" }"#;
        assert!(from_manifest(text, Path::new("C:\\"), Path::new("C:\\")).is_none());
        let server = r#""AppState" { "appid" "4020" "name" "Garry's Mod Dedicated Server" }"#;
        assert!(from_manifest(server, Path::new("C:\\"), Path::new("C:\\")).is_none());
    }

    #[test]
    fn only_installed_games_show() {
        let manifest = |flags: u32| format!(r#""AppState" {{ "appid" "620" "name" "Portal 2" "StateFlags" "{flags}" }}"#);
        let show = |flags| from_manifest(&manifest(flags), Path::new("C:\\"), Path::new("C:\\")).is_some();
        assert!(show(4));
        assert!(show(6), "an update waiting still lets it start");
        assert!(!show(1026), "still downloading");
        assert!(!show(4 | 2048), "being removed");
    }
}
