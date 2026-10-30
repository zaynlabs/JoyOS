use std::path::PathBuf;

use super::{Rgb, Source, Title};
use crate::system::registry;

/// Ubisoft Connect records each install under a key named by its product id.
const INSTALLS: &str = r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs";
/// Its uninstall entry has the real name, e.g. "Assassin's Creed Odyssey" for folder "ACOdyssey".
const UNINSTALL: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

pub fn scan() -> Vec<Title> {
    registry::local_machine_subkeys(INSTALLS)
        .into_iter()
        .filter_map(|id| {
            let dir = registry::read_local_machine_string(&format!(r"{INSTALLS}\{id}"), "InstallDir")?;
            let name = registry::read_local_machine_string(&format!(r"{UNINSTALL}\Uplay Install {id}"), "DisplayName");
            from_install(&id, PathBuf::from(dir), name)
        })
        .collect()
}

/// Without an uninstall entry, the install folder's name stands in for the title.
fn from_install(id: &str, dir: PathBuf, name: Option<String>) -> Option<Title> {
    if !dir.exists() {
        return None;
    }
    let name = match name.filter(|n| !n.trim().is_empty()) {
        Some(n) => n,
        None => dir.file_name()?.to_string_lossy().into_owned(),
    };
    Some(Title {
        id: format!("ubisoft:{id}"),
        launch: format!("uplay://launch/{id}/0"),
        name,
        source: Source::Ubisoft,
        emulator: None,
        args: None,
        cover: None,
        hero: None,
        logo: None,
        folder: Some(dir),
        last_played: None,
        accent: Rgb::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_name_becomes_the_title() {
        let dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let title = from_install("720", dir.clone(), None).unwrap();
        assert_eq!(title.id, "ubisoft:720");
        assert_eq!(title.launch, "uplay://launch/720/0");
        assert_eq!(title.folder, Some(dir.clone()));
        let named = from_install("720", dir, Some("Assassin's Creed Odyssey".into())).unwrap();
        assert_eq!(named.name, "Assassin's Creed Odyssey");
    }

    #[test]
    fn skips_removed_installs() {
        assert!(from_install("1", PathBuf::from(r"C:\definitely\not\here"), None).is_none());
    }
}
