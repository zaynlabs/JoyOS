//! Checks a manifest for a newer JoyOS. Nothing installs by itself: an update is
//! a page to open, and the user runs the installer.

use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    version: String,
    url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub version: String,
    pub url: String,
}

/// The newer release listed at `manifest_url`, if any. Offline or unset is `None`.
/// Test versions (like `1.3.0-rc.1`) only count when `test_versions` is on.
pub fn check(manifest_url: &str, test_versions: bool) -> Option<Update> {
    // A plain http manifest could be swapped on the way, pointing at someone else's download.
    if !manifest_url.trim().starts_with("https://") {
        return None;
    }
    let config = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(8))).build();
    let agent = ureq::Agent::new_with_config(config);
    let manifest: Manifest = agent.get(manifest_url.trim()).call().ok()?.body_mut().read_json().ok()?;
    if !newer(env!("CARGO_PKG_VERSION"), &manifest.version, test_versions) || !manifest.url.starts_with("https://") {
        return None;
    }
    Some(Update { version: manifest.version, url: manifest.url })
}

/// Whether `candidate` comes after `current`, by the usual version rules (0.10 beats 0.9).
pub fn newer(current: &str, candidate: &str, test_versions: bool) -> bool {
    let parse = |v: &str| semver::Version::parse(v.trim().trim_start_matches('v')).ok();
    match (parse(current), parse(candidate)) {
        (Some(current), Some(candidate)) => (test_versions || candidate.pre.is_empty()) && candidate > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(newer("0.9.0", "0.10.0", false));
        assert!(newer("0.1.0", "v0.1.1", false));
        assert!(!newer("1.2.0", "1.2.0", false));
        assert!(!newer("1.2.0", "1.1.9", false));
        assert!(!newer("1.2.0", "nonsense", false));
    }

    #[test]
    fn test_versions_only_when_asked() {
        assert!(!newer("1.2.0", "1.3.0-rc.1", false));
        assert!(newer("1.2.0", "1.3.0-rc.1", true));
        assert!(!newer("1.3.0", "1.3.0-rc.1", true), "a test version comes before its release");
    }

    #[test]
    fn only_https_manifests_are_read() {
        assert_eq!(check("  ", false), None);
        assert_eq!(check("http://example.com/joyos.json", false), None);
    }
}
