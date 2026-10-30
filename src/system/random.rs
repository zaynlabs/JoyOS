//! Random bytes from Windows' own generator, for PINs, salts and pairing tokens.

use windows::Win32::Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom};

/// `None` if Windows couldn't give us any. Callers must not fall back to zeros.
pub fn bytes(len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    let status = unsafe { BCryptGenRandom(None, &mut buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.is_ok() {
        Some(buf)
    } else {
        tracing::error!("no random bytes from Windows: {status:?}");
        None
    }
}

/// Lowercase hex, two characters per byte.
pub fn hex(len: usize) -> Option<String> {
    Some(bytes(len)?.iter().map(|b| format!("{b:02x}")).collect())
}

/// A number with `digits` digits, leading zeros kept.
pub fn digits(digits: u32) -> Option<String> {
    let max = 10u64.pow(digits);
    let n = bytes(8)?.iter().fold(0u64, |acc, b| acc << 8 | *b as u64) % max;
    Some(format!("{n:0width$}", width = digits as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_and_digits_have_the_right_shape() {
        assert_eq!(hex(16).unwrap().len(), 32);
        let pin = digits(6).unwrap();
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }
}
