//! ROM folders the user added in Settings. Each file is a game; the folder's
//! emulator runs it.

use std::path::{Path, PathBuf};

use super::{Rgb, Source, Title};
use crate::settings::RomFolder;

const EXTENSIONS: &[&str] = &[
    "nes", "sfc", "smc", "gb", "gbc", "gba", "nds", "3ds", "cia", "n64", "z64", "v64", "gcm", "iso", "wbfs", "cso", "chd",
    "cue", "bin", "pbp", "xci", "nsp", "md", "gen", "sms", "gg", "pce", "ngp", "ws", "vpk", "wad", "a26",
];

/// A folder with thousands of files shouldn't stall the scan.
const MAX_PER_FOLDER: usize = 2000;

pub fn scan(folders: &[RomFolder]) -> Vec<Title> {
    folders
        .iter()
        .filter(|f| !f.path.trim().is_empty())
        .flat_map(|f| {
            let Ok(entries) = std::fs::read_dir(&f.path) else { return vec![] };
            let paths = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && is_rom(p))
                .take(MAX_PER_FOLDER)
                .collect::<Vec<_>>();
            let paths = without_tracks(paths);
            paths.iter().filter_map(|p| to_title(p, &f.emulator, &f.args)).collect::<Vec<_>>()
        })
        .collect()
}

fn is_rom(path: &Path) -> bool {
    path.extension().is_some_and(|e| EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// A disc image comes as a `.cue` sheet plus its `.bin` tracks; only the sheet is a game.
fn without_tracks(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let has_cue = |p: &Path| p.with_extension("cue").is_file() || p.with_extension("CUE").is_file();
    paths.into_iter().filter(|p| !(p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bin")) && has_cue(p))).collect()
}

fn to_title(path: &Path, emulator: &str, args: &str) -> Option<Title> {
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let emulator = (!emulator.trim().is_empty()).then(|| emulator.to_owned());
    let args = (!args.trim().is_empty()).then(|| args.to_owned());
    Some(Title {
        id: format!("rom:{}", path.to_string_lossy()),
        launch: path.to_string_lossy().into_owned(),
        name,
        source: Source::Rom,
        emulator,
        args,
        cover: None,
        hero: None,
        logo: None,
        folder: path.parent().map(|p| p.to_path_buf()),
        last_played: None,
        accent: Rgb::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_rom_extensions_count() {
        assert!(is_rom(Path::new(r"D:\Roms\Zelda.SFC")));
        assert!(!is_rom(Path::new(r"D:\Roms\readme.txt")));
    }

    #[test]
    fn emulator_is_optional() {
        let with = to_title(Path::new(r"D:\Roms\Mario.nes"), "C:\\emu\\nes.exe", "").unwrap();
        assert_eq!(with.name, "Mario");
        assert_eq!(with.emulator.as_deref(), Some("C:\\emu\\nes.exe"));
        let without = to_title(Path::new(r"D:\Roms\Mario.nes"), "  ", "").unwrap();
        assert_eq!(without.emulator, None);
    }

    #[test]
    fn bin_tracks_hide_behind_their_cue_sheet() {
        let dir = std::env::temp_dir().join(format!("joyos-roms-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["Game.cue", "Game.bin", "Other.bin"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let kept = without_tracks(vec![dir.join("Game.cue"), dir.join("Game.bin"), dir.join("Other.bin")]);
        assert_eq!(kept, vec![dir.join("Game.cue"), dir.join("Other.bin")]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
