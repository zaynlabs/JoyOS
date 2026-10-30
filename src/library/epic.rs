use std::path::PathBuf;

use serde::Deserialize;

use super::{Rgb, Source, Title};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Manifest {
    display_name: String,
    app_name: String,
    catalog_namespace: String,
    catalog_item_id: String,
    install_location: String,
    #[serde(default)]
    app_categories: Vec<String>,
    #[serde(default, rename = "bIsIncompleteInstall")]
    incomplete: bool,
    /// Set on add-ons; they point at the game they belong to.
    #[serde(default)]
    main_game_app_name: Option<String>,
}

pub fn scan() -> Vec<Title> {
    let base = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let Ok(entries) = std::fs::read_dir(base.join(r"Epic\EpicGamesLauncher\Data\Manifests")) else {
        return vec![];
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("item")))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|text| from_manifest(&text))
        // Uninstalled outside the launcher leaves the manifest behind.
        .filter(|g| g.folder.as_ref().is_some_and(|f| f.is_dir()))
        .collect()
}

fn from_manifest(text: &str) -> Option<Title> {
    let m: Manifest = serde_json::from_str(text).ok()?;
    let is_game = m.app_categories.iter().any(|c| c == "games");
    let is_addon = m.main_game_app_name.as_ref().is_some_and(|main| *main != m.app_name);
    if m.incomplete || !is_game || is_addon {
        return None;
    }
    Some(Title {
        id: format!("epic:{}", m.app_name),
        launch: format!(
            "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true",
            m.catalog_namespace, m.catalog_item_id, m.app_name
        ),
        name: m.display_name,
        source: Source::Epic,
        emulator: None,
        args: None,
        cover: None,
        hero: None,
        logo: None,
        folder: Some(PathBuf::from(m.install_location)),
        last_played: None,
        accent: Rgb::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_item_file() {
        let text = r#"{"DisplayName":"Alan Wake 2","AppName":"Dill","CatalogNamespace":"ns","CatalogItemId":"item","InstallLocation":"D:\\Epic\\AlanWake2","AppCategories":["public","games","applications"]}"#;
        let game = from_manifest(text).unwrap();
        assert_eq!(game.id, "epic:Dill");
        assert!(game.launch.starts_with("com.epicgames.launcher://apps/ns%3Aitem%3ADill"));
    }

    #[test]
    fn skips_unfinished_installs() {
        let text = r#"{"DisplayName":"X","AppName":"x","CatalogNamespace":"n","CatalogItemId":"i","InstallLocation":"","bIsIncompleteInstall":true}"#;
        assert!(from_manifest(text).is_none());
    }

    #[test]
    fn skips_add_ons_and_uncategorised_items() {
        let addon = r#"{"DisplayName":"DLC","AppName":"dlc","MainGameAppName":"Dill","CatalogNamespace":"n","CatalogItemId":"i","InstallLocation":"","AppCategories":["games"]}"#;
        assert!(from_manifest(addon).is_none());
        let bare = r#"{"DisplayName":"X","AppName":"x","CatalogNamespace":"n","CatalogItemId":"i","InstallLocation":""}"#;
        assert!(from_manifest(bare).is_none());
    }
}
