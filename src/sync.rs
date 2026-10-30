//! Keeps settings the same on every PC through one shared folder (OneDrive, a
//! NAS, a USB stick). A copy only moves when its content changed since the last
//! sync, so a local save can't win just by being newer. If both PCs changed,
//! the newer file wins. Only the settings file moves; cover art and logs stay on each PC.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::settings::write_atomic;

const FILE: &str = "joyos-settings.toml";

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Pushed,
    Pulled,
    UpToDate,
}

/// `state` remembers what both sides looked like after the last sync.
pub fn sync(folder: &Path, local: &Path, state: &Path) -> Result<Outcome> {
    fs::create_dir_all(folder)?;
    let remote = folder.join(FILE);
    let (l, r) = (fs::read(local).ok(), fs::read(&remote).ok());
    let last = fs::read_to_string(state).ok();
    let outcome = match (&l, &r) {
        (None, None) => Outcome::UpToDate,
        (Some(_), None) => Outcome::Pushed,
        (None, Some(_)) => Outcome::Pulled,
        (Some(l), Some(r)) if hash(l) == hash(r) => Outcome::UpToDate,
        (Some(l), Some(_)) if last.as_deref() == Some(hash(l).as_str()) => Outcome::Pulled,
        (Some(_), Some(r)) if last.as_deref() == Some(hash(r).as_str()) => Outcome::Pushed,
        // Both changed, or we've never synced: the newer file wins.
        _ if modified(local) >= modified(&remote) => Outcome::Pushed,
        _ => Outcome::Pulled,
    };
    let now = match outcome {
        Outcome::Pushed => l.inspect(|bytes| {
            let _ = write_atomic(&remote, bytes);
        }),
        Outcome::Pulled => r.inspect(|bytes| {
            let _ = write_atomic(local, bytes);
        }),
        Outcome::UpToDate => l,
    };
    if let Some(bytes) = now {
        write_atomic(state, hash(&bytes).as_bytes())?;
    }
    Ok(outcome)
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("joyos-sync-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn at(path: &Path, text: &str, time: SystemTime) {
        fs::write(path, text).unwrap();
        fs::File::options().write(true).open(path).unwrap().set_modified(time).unwrap();
    }

    #[test]
    fn changes_move_both_ways() {
        let dir = scratch("both-ways");
        let (local, state, folder) = (dir.join("local.toml"), dir.join("state"), dir.join("shared"));
        let then = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);

        at(&local, "a = 1", then);
        assert_eq!(sync(&folder, &local, &state).unwrap(), Outcome::Pushed);
        assert_eq!(sync(&folder, &local, &state).unwrap(), Outcome::UpToDate);

        // The other PC changed it, even with an older clock: still pulled.
        at(&folder.join(FILE), "a = 2", then - Duration::from_secs(60));
        assert_eq!(sync(&folder, &local, &state).unwrap(), Outcome::Pulled);
        assert_eq!(fs::read_to_string(&local).unwrap(), "a = 2");

        // Only this PC changed: pushed, however old the time looks.
        at(&local, "a = 3", then - Duration::from_secs(600));
        assert_eq!(sync(&folder, &local, &state).unwrap(), Outcome::Pushed);
        assert_eq!(fs::read_to_string(folder.join(FILE)).unwrap(), "a = 3");

        // Both changed: the newer one wins.
        at(&local, "a = 4", then + Duration::from_secs(10));
        at(&folder.join(FILE), "a = 5", then + Duration::from_secs(20));
        assert_eq!(sync(&folder, &local, &state).unwrap(), Outcome::Pulled);
        let _ = fs::remove_dir_all(&dir);
    }
}
