use std::path::{Path, PathBuf};

use super::{Rgb, Source, Title, media_url, vdf};

/// Tools and runtimes Steam installs alongside games.
const NOT_GAMES: &[&str] = &["228980", "1070560", "1391110", "1628350", "1493710", "2180100"];

fn steam_root() -> Option<PathBuf> {
    crate::system::registry::read_string(r"Software\Valve\Steam", "SteamPath")
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .or_else(|| {
            let fallback = PathBuf::from(r"C:\Program Files (x86)\Steam");
            fallback.exists().then_some(fallback)
        })
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
    if NOT_GAMES.contains(&id.as_str())
        || name.contains("Redistributable")
        || name.starts_with("Proton")
        || name.starts_with("Steam Linux Runtime")
    {
        return None;
    }
    let art = |file: &str| find_art(cache, &id, file);
    Some(Title {
        launch: format!("steam://rungameid/{id}"),
        cover: art("library_600x900.jpg").or_else(|| Some(cdn(&id, "library_600x900.jpg"))),
        hero: art("library_hero.jpg").or_else(|| Some(cdn(&id, "library_hero.jpg"))),
        logo: art("logo.png"),
        folder: app.str("installdir").map(|d| lib.join("steamapps/common").join(d)),
        last_played: app.str("LastPlayed").and_then(|s| s.parse().ok()).filter(|&t: &i64| t > 0),
        accent: Rgb::default(),
        id: format!("steam:{id}"),
        name,
        source: Source::Steam,
    })
}

fn cdn(id: &str, file: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{id}/{file}")
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
        assert!(game.cover.unwrap().contains("/620/library_600x900.jpg"));
    }

    #[test]
    fn skips_runtimes() {
        let text = r#""AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" }"#;
        assert!(from_manifest(text, Path::new("C:\\"), Path::new("C:\\")).is_none());
    }
}
