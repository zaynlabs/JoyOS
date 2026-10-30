use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    /// Light by day, dark from 19:00 to 07:00.
    Auto,
}

/// Which on-screen keyboard opens over other apps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Keyboard {
    /// Our own, driven by the controller like the rest of JoyOS.
    #[default]
    Joy,
    Windows,
}

/// The moving background behind every screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scene {
    #[default]
    Still,
    Aurora,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode {
    #[default]
    Balanced,
    Performance,
    Saver,
}

/// Per-game overrides, applied when the game is launched and undone when JoyOS comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GameProfile {
    pub swap_ab: Option<bool>,
    pub power: Option<PowerMode>,
}

/// Someone in the household. Members can have a PIN and a daily play limit.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Member {
    pub id: String,
    pub name: String,
    /// Argon2id of the PIN, as `argon2$salt$hash`. Older files have salted SHA-256, `salt$hash`.
    pub pin_hash: Option<String>,
    pub daily_minutes: Option<u32>,
}

/// One stretch of play, from launch until JoyOS is called back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Session {
    pub member: Option<String>,
    pub game: String,
    /// Unix seconds.
    pub start: i64,
    pub seconds: u32,
    /// Older files counted whole minutes. Read, never written.
    #[serde(skip_serializing_if = "is_zero")]
    pub minutes: u32,
}

impl Session {
    pub fn secs(&self) -> u32 {
        self.seconds + self.minutes * 60
    }
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A website or app the user added to the media hub.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WebTile {
    pub name: String,
    pub url: String,
}

/// A folder of ROMs and the emulator that plays them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RomFolder {
    pub path: String,
    pub emulator: String,
    /// What the emulator is given. `{file}` is the game; empty means just the file.
    pub args: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `None` until the first-run setup picks one.
    pub language: Option<String>,
    pub theme: Theme,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub scene: Scene,
    pub sounds: bool,
    pub swap_ab: bool,
    /// 1 (slow) to 10 (fast).
    pub mouse_speed: u8,
    pub rumble: bool,
    pub keyboard: Keyboard,
    pub set_up: bool,
    /// Offers test versions in the update check too.
    pub test_versions: bool,
    /// Game ids pinned to Home, in order.
    pub pinned: Vec<String>,
    /// Unix seconds, keyed by game id. Fills in for stores that don't record it.
    pub played: HashMap<String, i64>,
    pub profiles: HashMap<String, GameProfile>,
    pub members: Vec<Member>,
    /// Member id of whoever is playing now; `None` means everyone.
    pub playing: Option<String>,
    pub sessions: Vec<Session>,
    /// Words typed on the keyboard, most recent first, for suggestions.
    pub words: Vec<String>,
    pub web_tiles: Vec<WebTile>,
    pub roms: Vec<RomFolder>,
    /// SteamGridDB API key for cover art. Empty means no online art.
    pub art_key: String,
    /// Lets a phone on the same network act as a remote.
    pub phone_remote: bool,
    /// Runs `plugins/*/plugin.js` in the shell.
    pub plugins: bool,
    /// Signed key from the store or a purchase. Empty on the free plan.
    pub license: String,
    /// Folder (for example in OneDrive) where settings sync between PCs.
    pub sync_folder: Option<String>,
    /// JSON manifest that says which version is current. Empty turns update checks off.
    pub update_url: String,
    /// Writes a local crash log. Nothing is ever uploaded.
    pub crash_reports: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: None,
            theme: Theme::Dark,
            reduce_motion: false,
            high_contrast: false,
            large_text: false,
            scene: Scene::Still,
            sounds: true,
            swap_ab: false,
            mouse_speed: 5,
            rumble: true,
            keyboard: Keyboard::Joy,
            set_up: false,
            test_versions: false,
            pinned: vec![],
            played: HashMap::new(),
            profiles: HashMap::new(),
            members: vec![],
            playing: None,
            sessions: vec![],
            words: vec![],
            web_tiles: vec![],
            roms: vec![],
            art_key: String::new(),
            phone_remote: false,
            plugins: false,
            license: String::new(),
            sync_folder: None,
            update_url: String::new(),
            crash_reports: false,
        }
    }
}

/// The folder JoyOS keeps its files in: settings, art, plugins, logs.
pub fn data_root() -> Option<PathBuf> {
    ProjectDirs::from("app", "JoyOS", "JoyOS").map(|d| d.data_local_dir().to_path_buf())
}

fn path() -> Option<PathBuf> {
    ProjectDirs::from("app", "JoyOS", "JoyOS").map(|d| d.config_dir().join("settings.toml"))
}

/// Where the settings file lives, so sync can compare timestamps.
pub fn file_path() -> Option<PathBuf> {
    path()
}

/// Set when the settings file exists but couldn't be read, so we never write over it.
static LOCKED: AtomicBool = AtomicBool::new(false);

impl Settings {
    pub fn load() -> Self {
        path().map(|p| Self::load_from(&p)).unwrap_or_default()
    }

    fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(s) => s,
                Err(e) => {
                    // Keep the broken file next to the new one, so nothing is lost for good.
                    let backup = path.with_file_name(format!("settings.broken-{}.toml", crate::app::now()));
                    tracing::warn!("settings didn't parse ({e}); kept a copy at {}", backup.display());
                    if std::fs::copy(path, &backup).is_err() {
                        LOCKED.store(true, Ordering::Relaxed);
                    }
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                tracing::warn!("couldn't read settings ({e}); changes won't be saved this time");
                LOCKED.store(true, Ordering::Relaxed);
                Self::default()
            }
        }
    }

    /// Writes now. Most callers want `save_soon`.
    pub fn save(&self) {
        if LOCKED.load(Ordering::Relaxed) {
            return;
        }
        let Some(path) = path() else { return };
        let result = toml::to_string_pretty(self).map_err(anyhow::Error::from).and_then(|text| write_atomic(&path, text.as_bytes()));
        if let Err(e) = result {
            tracing::warn!("couldn't save settings: {e}");
        }
    }
}

/// Writes beside the target and renames over it, so a crash never leaves half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// The newest unsaved settings, and whether a write is in progress.
static PENDING: Mutex<Option<Settings>> = Mutex::new(None);
static WAKE: Condvar = Condvar::new();
static WRITING: Mutex<()> = Mutex::new(());

/// Saves a moment later, on a thread. A burst of changes becomes one write.
pub fn save_soon(s: Settings) {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let _ = std::thread::Builder::new().name("settings".into()).spawn(|| loop {
            {
                let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
                while pending.is_none() {
                    pending = WAKE.wait(pending).unwrap_or_else(|e| e.into_inner());
                }
            }
            std::thread::sleep(Duration::from_millis(500));
            flush();
        });
    });
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
    WAKE.notify_one();
}

/// Writes any unsaved settings right away, e.g. before a sync or on exit.
pub fn flush() {
    let _writing = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let pending = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(s) = pending {
        s.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_missing_fields() {
        let mut s = Settings::default();
        s.pinned.push("steam:620".into());
        s.keyboard = Keyboard::Windows;
        s.profiles.insert("steam:620".into(), GameProfile { swap_ab: Some(true), power: Some(PowerMode::Performance) });
        s.members.push(Member { id: "m1".into(), name: "Sam".into(), pin_hash: None, daily_minutes: Some(90) });
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        let partial: Settings = toml::from_str("sounds = false").unwrap();
        assert!(!partial.sounds);
        assert_eq!(partial.mouse_speed, 5);
        assert_eq!(partial.keyboard, Keyboard::Joy);
        assert!(partial.members.is_empty());
    }

    #[test]
    fn old_minutes_still_count() {
        let s: Session = toml::from_str("game = \"x\"
start = 1
minutes = 2").unwrap();
        assert_eq!(s.secs(), 120);
        let text = toml::to_string(&Session { seconds: 5, ..Default::default() }).unwrap();
        assert!(!text.contains("minutes"));
    }

    #[test]
    fn a_broken_file_is_kept_and_atomic_writes_replace() {
        let dir = std::env::temp_dir().join(format!("joyos-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.toml");
        std::fs::write(&file, "sounds = [not toml").unwrap();
        assert_eq!(Settings::load_from(&file), Settings::default());
        let kept = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).any(|e| e.file_name().to_string_lossy().starts_with("settings.broken-"));
        assert!(kept, "the unreadable file is copied aside");
        write_atomic(&file, b"sounds = false").unwrap();
        write_atomic(&file, b"sounds = true").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "sounds = true");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
