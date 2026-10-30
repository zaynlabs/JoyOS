use std::path::PathBuf;

use super::{Rgb, Source, Title};
use crate::system::registry;

/// GOG Galaxy writes one subkey per installed game here (32-bit view on 64-bit Windows).
const GAMES_KEY: &str = r"SOFTWARE\WOW6432Node\GOG.com\Games";

pub fn scan() -> Vec<Title> {
    registry::local_machine_subkeys(GAMES_KEY)
        .into_iter()
        .filter_map(|id| {
            let key = format!(r"{GAMES_KEY}\{id}");
            let read = |value: &str| registry::read_local_machine_string(&key, value).filter(|v| !v.trim().is_empty());
            // Add-ons are listed too; they name the game they belong to.
            if read("dependsOn").is_some() {
                return None;
            }
            let name = read("gameName")?;
            let exe = read("exe")?;
            let folder = read("workingDir").or_else(|| read("path"));
            let mut game = to_title(&id, name, PathBuf::from(exe), folder.map(PathBuf::from))?;
            game.args = read("launchParam");
            Some(game)
        })
        .collect()
}

/// Skips entries whose executable is gone (uninstalled but not cleaned up).
fn to_title(id: &str, name: String, exe: PathBuf, folder: Option<PathBuf>) -> Option<Title> {
    if name.is_empty() || !exe.exists() {
        return None;
    }
    Some(Title {
        id: format!("gog:{id}"),
        launch: exe.to_string_lossy().into_owned(),
        name,
        source: Source::Gog,
        emulator: None,
        args: None,
        cover: None,
        hero: None,
        logo: None,
        folder,
        last_played: None,
        accent: Rgb::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_title_from_installed_game() {
        let exe = std::env::current_exe().unwrap();
        let game = to_title("1207658924", "The Witcher".into(), exe.clone(), None).unwrap();
        assert_eq!(game.id, "gog:1207658924");
        assert_eq!(game.launch, exe.to_string_lossy());
        assert_eq!(game.source, Source::Gog);
    }

    #[test]
    fn skips_missing_executables() {
        let missing = PathBuf::from(r"C:\definitely\not\here\game.exe");
        assert!(to_title("1", "Gone".into(), missing, None).is_none());
    }
}
