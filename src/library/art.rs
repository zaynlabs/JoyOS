//! Cover art from SteamGridDB. Every game is looked up once; the result (or a
//! "no art" marker) is cached on disk, so later launches don't ask again.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{Source, Title, media_url};
use crate::settings;

const API: &str = "https://www.steamgriddb.com/api/v2";
/// Look-ups at once. Enough to be quick, few enough to be polite to the site.
const WORKERS: usize = 4;

enum Lookup {
    Found(Vec<u8>, &'static str),
    /// The site answered and has nothing for this title.
    Missing,
    /// No network, or the site errored. Try again next time.
    Offline,
    /// The site turned the key down.
    BadKey,
}

/// Why online art didn't work, worth telling the user about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtError {
    BadKey,
}

fn folder() -> Option<PathBuf> {
    let dir = settings::data_root()?.join("art");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Covers saved on an earlier run. Only the disk; quick enough for the first scan.
pub fn fill_cached(titles: &mut [Title]) {
    let Some(dir) = folder() else { return };
    for title in titles.iter_mut().filter(|t| t.cover.is_none()) {
        if let Some(file) = cached(&dir, &safe_name(&title.id)) {
            title.cover = Some(media_url(&file));
        }
    }
}

/// Fills in `cover` for titles that don't have art. Without a key, or offline,
/// titles keep whatever their store gave them.
pub fn fill(titles: &mut [Title], key: &str) -> Option<ArtError> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let dir = folder()?;
    forget_misses_if_key_changed(&dir, key);
    let todo: Vec<usize> = (0..titles.len())
        .filter(|&i| titles[i].cover.is_none())
        .filter(|&i| !dir.join(format!("{}.none", safe_name(&titles[i].id))).exists())
        .collect();
    let queue = Mutex::new(todo.into_iter());
    let found = Mutex::new(vec![]);
    let bad_key = std::sync::atomic::AtomicBool::new(false);
    let titles_ref = &*titles;
    std::thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| {
                loop {
                    if bad_key.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    let Some(i) = queue.lock().unwrap_or_else(|e| e.into_inner()).next() else { return };
                    let title = &titles_ref[i];
                    let stem = safe_name(&title.id);
                    match lookup(title, key) {
                        Lookup::Found(bytes, ext) => {
                            let file = dir.join(format!("{stem}.{ext}"));
                            if settings::write_atomic(&file, &bytes).is_ok() {
                                found.lock().unwrap_or_else(|e| e.into_inner()).push((i, media_url(&file)));
                            }
                        }
                        Lookup::Missing => {
                            let _ = std::fs::write(dir.join(format!("{stem}.none")), b"");
                        }
                        Lookup::BadKey => bad_key.store(true, std::sync::atomic::Ordering::Relaxed),
                        Lookup::Offline => {}
                    }
                }
            });
        }
    });
    for (i, url) in found.into_inner().unwrap_or_else(|e| e.into_inner()) {
        titles[i].cover = Some(url);
    }
    bad_key.into_inner().then_some(ArtError::BadKey)
}

/// "Nothing found" was true for the old key's account; a new key deserves a fresh look.
fn forget_misses_if_key_changed(dir: &Path, key: &str) {
    let marker = dir.join("key");
    let hash: String = Sha256::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    if std::fs::read_to_string(&marker).ok().as_deref() == Some(hash.as_str()) {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "none")) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let _ = std::fs::write(marker, hash);
}

/// A file already saved for this title, with either image extension.
fn cached(dir: &Path, stem: &str) -> Option<PathBuf> {
    ["png", "jpg"].iter().map(|ext| dir.join(format!("{stem}.{ext}"))).find(|p| p.exists())
}

fn lookup(title: &Title, key: &str) -> Lookup {
    // Steam games have an exact id; a name search can pick the wrong game.
    let steam_id = (title.source == Source::Steam).then(|| title.id.trim_start_matches("steam:"));
    let game = match steam_id {
        Some(id) => format!("steam/{id}"),
        None => {
            let search = format!("{API}/search/autocomplete/{}", encode(&title.name));
            let found = match get_json(&search, key) {
                Ok(v) => v,
                Err(e) => return e,
            };
            let Some(game) = found["data"].get(0).and_then(|g| g["id"].as_u64()) else { return Lookup::Missing };
            format!("game/{game}")
        }
    };

    // `grids/steam/<appid>` or `grids/game/<id>`.
    let grids = format!("{API}/grids/{game}?dimensions=600x900&mimes=image/png,image/jpeg&types=static");
    let grids = match get_json(&grids, key) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(url) = grids["data"].get(0).and_then(|g| g["url"].as_str()) else { return Lookup::Missing };

    let ext = if url.to_ascii_lowercase().ends_with(".jpg") || url.to_ascii_lowercase().ends_with(".jpeg") {
        "jpg"
    } else {
        "png"
    };
    match agent().get(url).call().ok().and_then(|mut r| r.body_mut().read_to_vec().ok()) {
        Some(bytes) => Lookup::Found(bytes, ext),
        None => Lookup::Offline,
    }
}

fn get_json(url: &str, key: &str) -> Result<Value, Lookup> {
    match agent().get(url).header("Authorization", &format!("Bearer {key}")).call() {
        Ok(mut resp) => resp.body_mut().read_json::<Value>().map_err(|_| Lookup::Offline),
        Err(ureq::Error::StatusCode(401 | 403)) => Err(Lookup::BadKey),
        // An unknown Steam id is a 404: nothing to find, not a fault.
        Err(ureq::Error::StatusCode(404)) => Err(Lookup::Missing),
        Err(e) => {
            tracing::debug!("art look-up failed: {e}");
            Err(Lookup::Offline)
        }
    }
}

/// One shared client with a timeout, so a slow site can't hold up the library scan.
fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let config = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(8))).build();
        ureq::Agent::new_with_config(config)
    })
}

fn safe_name(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_safe_and_encoded() {
        assert_eq!(safe_name("steam:620"), "steam_620");
        assert_eq!(encode("Half-Life 2: Episode"), "Half-Life%202%3A%20Episode");
    }

    #[test]
    fn no_key_means_no_work() {
        let mut titles = vec![];
        assert_eq!(fill(&mut titles, "   "), None);
        assert!(titles.is_empty());
    }
}
