use std::{collections::HashMap, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `None` until the first-run setup picks one.
    pub language: Option<String>,
    pub theme: Theme,
    pub reduce_motion: bool,
    pub sounds: bool,
    pub swap_ab: bool,
    /// 1 (slow) to 10 (fast).
    pub mouse_speed: u8,
    pub rumble: bool,
    pub keyboard: Keyboard,
    pub set_up: bool,
    /// Game ids pinned to Home, in order.
    pub pinned: Vec<String>,
    /// Unix seconds, keyed by game id. Fills in for stores that don't record it.
    pub played: HashMap<String, i64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: None,
            theme: Theme::Dark,
            reduce_motion: false,
            sounds: true,
            swap_ab: false,
            mouse_speed: 5,
            rumble: true,
            keyboard: Keyboard::Joy,
            set_up: false,
            pinned: vec![],
            played: HashMap::new(),
        }
    }
}

fn path() -> Option<PathBuf> {
    ProjectDirs::from("app", "JoyOS", "JoyOS").map(|d| d.config_dir().join("settings.toml"))
}

impl Settings {
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = path() else { return };
        let result = (|| -> anyhow::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, toml::to_string_pretty(self)?)?;
            Ok(())
        })();
        if let Err(e) = result {
            tracing::warn!("couldn't save settings: {e}");
        }
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
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        let partial: Settings = toml::from_str("sounds = false").unwrap();
        assert!(!partial.sounds);
        assert_eq!(partial.mouse_speed, 5);
        assert_eq!(partial.keyboard, Keyboard::Joy);
    }
}
