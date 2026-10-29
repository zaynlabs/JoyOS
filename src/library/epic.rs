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
        .collect()
}

fn from_manifest(text: &str) -> Option<Title> {
    let m: Manifest = serde_json::from_str(text).ok()?;
    let is_game = m.app_categories.is_empty() || m.app_categories.iter().any(|c| c == "games");
    if m.incomplete || !is_game {
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
}
