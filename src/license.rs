//! Plans and license keys. A key is `JOY1.<payload>.<signature>`: the payload is
//! JSON, and the signature is ed25519 over it. Checking is offline; nothing phones home.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;

/// Public half of the key that signs licenses. Made by `tools/joyos-license.mjs keygen`.
const PUBLIC_KEY: [u8; 32] = [
    0x59, 0xfb, 0x4f, 0x5e, 0xfc, 0x7a, 0xfd, 0xa3, 0x55, 0xd9, 0x78, 0x20, 0xb0, 0xac, 0xde, 0xe7, 0x7d, 0x33, 0xfe,
    0x42, 0xaa, 0xf1, 0xca, 0x66, 0xbd, 0x6f, 0x6e, 0x2d, 0xce, 0x51, 0x0e, 0x25,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    #[default]
    Free,
    /// Themes, per-game settings, household profiles, cloud sync, plugins.
    Plus,
    /// Everything in Plus, plus kiosk setup and seat counts for venues.
    Business,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct License {
    pub plan: Plan,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub seats: u32,
}

/// The license in `key`, if it's well-formed and signed by us.
pub fn check(key: &str) -> Option<License> {
    verify(key, &PUBLIC_KEY)
}

pub fn plan(key: &str) -> Plan {
    check(key).map_or(Plan::Free, |l| l.plan)
}

/// Whether the plan covers a Plus feature.
pub fn has_plus(key: &str) -> bool {
    plan(key) >= Plan::Plus
}

fn verify(key: &str, public: &[u8; 32]) -> Option<License> {
    let mut parts = key.trim().split('.');
    if parts.next()? != "JOY1" {
        return None;
    }
    let payload = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;
    let sig: [u8; 64] = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?.try_into().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let key = VerifyingKey::from_bytes(public).ok()?;
    key.verify(&payload, &Signature::from_bytes(&sig)).ok()?;
    serde_json::from_slice(&payload).ok()
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    /// A throwaway key for tests, so the real signing key never has to be in the repo.
    fn test_key() -> (SigningKey, [u8; 32]) {
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let public = signing.verifying_key().to_bytes();
        (signing, public)
    }

    fn issue(signing: &SigningKey, payload: &str) -> String {
        let sig = signing.sign(payload.as_bytes());
        format!("JOY1.{}.{}", URL_SAFE_NO_PAD.encode(payload), URL_SAFE_NO_PAD.encode(sig.to_bytes()))
    }

    #[test]
    fn a_signed_key_unlocks_its_plan() {
        let (signing, public) = test_key();
        let key = issue(&signing, r#"{"plan":"plus","email":"a@b.c","seats":1}"#);
        let license = verify(&key, &public).unwrap();
        assert_eq!(license.plan, Plan::Plus);
        assert_eq!(license.email, "a@b.c");
    }

    #[test]
    fn tampering_and_strangers_are_rejected() {
        let (signing, public) = test_key();
        let key = issue(&signing, r#"{"plan":"plus"}"#);
        // Same signature, payload swapped for a higher plan.
        let signature = key.rsplit('.').next().unwrap();
        let forged = format!("JOY1.{}.{signature}", URL_SAFE_NO_PAD.encode(r#"{"plan":"business"}"#));
        assert!(verify(&forged, &public).is_none());
        assert!(verify("JOY2.x.y", &public).is_none());
        assert!(verify("", &public).is_none());
        let other = SigningKey::from_bytes(&[9u8; 32]);
        assert!(verify(&issue(&other, r#"{"plan":"plus"}"#), &public).is_none());
    }

    #[test]
    fn no_key_is_the_free_plan() {
        assert_eq!(plan(""), Plan::Free);
        assert!(!has_plus("nonsense"));
    }
}
