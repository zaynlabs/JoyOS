use std::path::{Path, PathBuf};

use super::{Rgb, Source, Title};

/// Shortcuts that aren't things people want to open from the couch.
const SKIP_WORDS: &[&str] = &[
    "uninstall", "help", "readme", "read me", "manual", "documentation", "website", "release notes",
    "license", "support", "changelog", "what's new", "debug", "safe mode", "setup", "updater",
    "administrative tools", "windows tools", "control panel", "recovery drive", "odbc",
];
const SKIP_FOLDERS: &[&str] = &[
    "administrative tools", "windows tools", "accessibility", "windows powershell", "system tools",
    "startup", "maintenance", "windows accessories", "windows administrative tools",
];

/// Apps people most often want on a TV. These float to the top.
const FAVOURITES: &[&str] = &[
    "google chrome", "microsoft edge", "firefox", "brave", "opera", "spotify", "discord", "netflix",
    "vlc media player", "steam", "epic games launcher", "xbox", "obs studio", "plex", "kodi",
];

pub fn scan() -> Vec<Title> {
    let mut roots = vec![];
    if let Some(p) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(p) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    let mut found = vec![];
    for root in roots {
        walk(&root, 0, &mut found);
    }

    let mut apps: Vec<Title> = vec![];
    for path in found {
        let Some(name) = path.file_stem().map(|s| s.to_string_lossy().trim().to_owned()) else { continue };
        if apps.iter().any(|a| a.name.eq_ignore_ascii_case(&name)) {
            continue;
        }
        apps.push(Title {
            id: format!("app:{}", name.to_lowercase()),
            launch: path.to_string_lossy().into_owned(),
            name,
            source: Source::App,
            emulator: None,
            args: None,
            cover: None,
            hero: None,
            logo: None,
            folder: None,
            last_played: None,
            accent: Rgb::default(),
        });
    }

    // File Explorer has no shortcut file on Windows 11, so add it by hand.
    apps.push(Title {
        id: "app:files".into(),
        name: crate::t!("apps-files"),
        source: Source::App,
        launch: "explorer.exe".into(),
        emulator: None,
        args: None,
        cover: None,
        hero: None,
        logo: None,
        folder: None,
        last_played: None,
        accent: Rgb::default(),
    });

    apps.sort_by_key(|a| {
        let lower = a.name.to_lowercase();
        let rank = FAVOURITES.iter().position(|f| lower == *f).unwrap_or(FAVOURITES.len());
        (rank, lower)
    });
    apps
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let lower = entry.file_name().to_string_lossy().to_lowercase();
        if path.is_dir() {
            if depth < 2 && !SKIP_FOLDERS.contains(&lower.as_str()) {
                walk(&path, depth + 1, out);
            }
        } else if let Some(stem) = lower.strip_suffix(".lnk")
            && !skipped(stem)
        {
            out.push(path);
        }
    }
}

/// Whole words only, so "Helpdesk" or "Setup Wizard Pro" style names aren't caught by "help".
fn skipped(name: &str) -> bool {
    let words = |s: &str| format!(" {} ", s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "));
    let name = words(name);
    SKIP_WORDS.iter().any(|w| name.contains(&words(w)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_words_match_whole_words() {
        assert!(skipped("uninstall foo"));
        assert!(skipped("foo help"));
        assert!(skipped("what's new in foo"));
        assert!(!skipped("helpdesk"));
        assert!(!skipped("setupper"));
    }
}
