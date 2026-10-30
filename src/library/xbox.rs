//! Game Pass for PC installs into an `XboxGames` folder, one folder per title. Each
//! title has a `Content\MicrosoftGame.config` that names it and its executable.

use std::path::{Path, PathBuf};

use super::{Rgb, Source, Title};

pub fn scan() -> Vec<Title> {
    roots()
        .iter()
        .filter_map(|root| std::fs::read_dir(root).ok())
        .flat_map(|entries| entries.flatten().filter_map(|e| from_folder(&e.path())).collect::<Vec<_>>())
        .collect()
}

/// Every drive Xbox installs to has a `.GamingRoot` file naming the folder it uses.
fn roots() -> Vec<PathBuf> {
    let mut roots = vec![];
    for letter in b'C'..=b'Z' {
        let drive = PathBuf::from(format!("{}:\\", letter as char));
        let Ok(bytes) = std::fs::read(drive.join(".GamingRoot")) else { continue };
        for folder in gaming_root_folders(&bytes) {
            let path = drive.join(folder);
            if !roots.contains(&path) {
                roots.push(path);
            }
        }
    }
    let default = PathBuf::from(r"C:\XboxGames");
    if !roots.contains(&default) {
        roots.push(default);
    }
    roots
}

/// The file is "RGBX", a count, then folder names in UTF-16, each ending in a zero.
fn gaming_root_folders(bytes: &[u8]) -> Vec<String> {
    if bytes.len() < 8 || &bytes[..4] != b"RGBX" {
        return vec![];
    }
    let wide: Vec<u16> = bytes[8..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    wide.split(|&c| c == 0)
        .map(String::from_utf16_lossy)
        .map(|s| s.trim_start_matches(['\\', '/']).to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn from_folder(folder: &Path) -> Option<Title> {
    let content = folder.join("Content");
    let config = std::fs::read_to_string(content.join("MicrosoftGame.config")).ok()?;
    let folder_name = folder.file_name()?.to_string_lossy().into_owned();
    // Some games point at a translated name (`ms-resource:…`) we can't read; the folder name is better.
    let name = attribute(&config, "ShellVisuals", "DefaultDisplayName")
        .filter(|n| !n.starts_with("ms-resource:"))
        .unwrap_or_else(|| folder_name.clone());
    let exe = attribute(&config, "Executable", "Name")?;
    to_title(&folder_name, &name, &content.join(exe))
}

fn to_title(id: &str, name: &str, exe: &Path) -> Option<Title> {
    if name.is_empty() || !exe.exists() {
        return None;
    }
    Some(Title {
        id: format!("xbox:{id}"),
        launch: exe.to_string_lossy().into_owned(),
        name: name.to_owned(),
        source: Source::Xbox,
        emulator: None,
        args: None,
        cover: None,
        hero: None,
        logo: None,
        folder: exe.parent().map(PathBuf::from),
        last_played: None,
        accent: Rgb::default(),
    })
}

/// Reads `name="…"` from the first `<tag …>` in an XML file. This is a
/// one-attribute lookup, not an XML parser.
fn attribute(xml: &str, tag: &str, name: &str) -> Option<String> {
    // The space matters: `<Executable` would also match `<ExecutableList>`.
    let start = xml.find(&format!("<{tag} "))?;
    let rest = &xml[start..];
    let rest = &rest[..rest.find('>')?];
    let at = rest.find(&format!(" {name}=\""))?;
    let value = &rest[at + name.len() + 3..];
    Some(value[..value.find('"')?].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_title_and_executable() {
        let xml = r#"<Game><ExecutableList><Executable Name="Game.exe" TargetDeviceFamily="PC"/></ExecutableList>
            <ShellVisuals DefaultDisplayName="Forza Horizon" /></Game>"#;
        assert_eq!(attribute(xml, "ShellVisuals", "DefaultDisplayName").as_deref(), Some("Forza Horizon"));
        assert_eq!(attribute(xml, "Executable", "Name").as_deref(), Some("Game.exe"));
        assert_eq!(attribute(xml, "Missing", "Name"), None);
    }

    #[test]
    fn reads_the_gaming_root_file() {
        let mut bytes = b"RGBX\x01\x00\x00\x00".to_vec();
        for c in "\\XboxGames".encode_utf16().chain([0]) {
            bytes.extend(c.to_le_bytes());
        }
        assert_eq!(gaming_root_folders(&bytes), vec!["XboxGames".to_owned()]);
        assert!(gaming_root_folders(b"nope").is_empty());
    }

    #[test]
    fn skips_missing_executables() {
        assert!(to_title("x", "Gone", Path::new(r"C:\definitely\not\here.exe")).is_none());
    }
}
