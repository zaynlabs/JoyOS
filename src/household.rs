//! Who's playing: PINs, daily play limits, and the stats that come from play sessions.
//! This is for families sharing a PC. A PIN keeps a child out of a profile; it isn't a security boundary.

use std::sync::Mutex;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::settings::{Member, Session};
use crate::system::random;

/// Most sessions kept. Older ones are dropped so the settings file stays small.
pub const MAX_SESSIONS: usize = 1000;

/// Wrong PINs allowed before the pad waits, and how long it waits.
const TRIES: u32 = 5;
const WAIT: i64 = 30;

/// A new Argon2id hash for `pin`, stored as `argon2$salt$hash`. `None` if Windows gave no random bytes.
pub fn new_pin_hash(pin: &str) -> Option<String> {
    let salt = random::hex(16)?;
    Some(format!("argon2${salt}${}", argon2_hex(&salt, pin)?))
}

pub fn check_pin(stored: &str, pin: &str) -> bool {
    let (expected, actual) = match stored.split('$').collect::<Vec<_>>()[..] {
        ["argon2", salt, hash] => (hash.to_owned(), argon2_hex(salt, pin).unwrap_or_default()),
        [salt, hash] => (hash.to_owned(), sha_hex(salt, pin)),
        _ => return false,
    };
    !actual.is_empty() && bool::from(expected.as_bytes().ct_eq(actual.as_bytes()))
}

/// Whether a stored hash is the old kind, so it gets replaced the next time the PIN is right.
pub fn is_old_hash(stored: &str) -> bool {
    !stored.starts_with("argon2$")
}

fn argon2_hex(salt: &str, pin: &str) -> Option<String> {
    let mut out = [0u8; 32];
    argon2::Argon2::default().hash_password_into(pin.as_bytes(), salt.as_bytes(), &mut out).ok()?;
    Some(out.iter().map(|b| format!("{b:02x}")).collect())
}

fn sha_hex(salt: &str, pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(pin.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Wrong tries in a row, and when the pad opens again.
static MISSES: Mutex<(u32, i64)> = Mutex::new((0, 0));

/// Seconds until another PIN can be tried; 0 when it can be tried now.
pub fn pin_wait(now: i64) -> i64 {
    let misses = MISSES.lock().unwrap_or_else(|e| e.into_inner());
    (misses.1 - now).max(0)
}

/// Counts a try. After too many wrong ones in a row, the pad waits a bit.
pub fn pin_tried(ok: bool, now: i64) {
    let mut misses = MISSES.lock().unwrap_or_else(|e| e.into_inner());
    if ok {
        *misses = (0, 0);
    } else {
        misses.0 += 1;
        if misses.0 >= TRIES {
            *misses = (0, now + WAIT);
        }
    }
}

/// Seconds since local midnight, so daily limits reset at the start of the day.
pub fn seconds_into_local_day() -> i64 {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    t.wHour as i64 * 3600 + t.wMinute as i64 * 60 + t.wSecond as i64
}

pub fn member<'a>(members: &'a [Member], id: &str) -> Option<&'a Member> {
    members.iter().find(|m| m.id == id)
}

/// How far local time is ahead of UTC, in seconds.
pub fn local_offset(now: i64) -> i64 {
    let off = (seconds_into_local_day() - now.rem_euclid(86_400)).rem_euclid(86_400);
    // Offsets run from -12h to +14h; anything past +14h is really behind UTC.
    if off > 14 * 3600 { off - 86_400 } else { off }
}

/// Cuts a stretch of play at each local midnight, so every day gets its own share.
pub fn split_by_day(start: i64, seconds: u32, offset: i64) -> Vec<(i64, u32)> {
    let mut out = vec![];
    let (mut at, end) = (start, start + seconds as i64);
    while at < end {
        let next_midnight = at - (at + offset).rem_euclid(86_400) + 86_400;
        let until = next_midnight.min(end);
        out.push((at, (until - at) as u32));
        at = until;
    }
    out
}

/// Minutes played since `since`, for one member, or for everyone when `who` is `None`.
pub fn minutes_since(sessions: &[Session], who: Option<&str>, since: i64) -> u32 {
    sessions.iter().filter(|s| matches(s, who) && s.start >= since).map(|s| s.secs()).sum::<u32>() / 60
}

/// The most played games since `since`, most minutes first.
pub fn top_games(sessions: &[Session], who: Option<&str>, since: i64, count: usize) -> Vec<(String, u32)> {
    let mut totals: Vec<(String, u32)> = vec![];
    for s in sessions.iter().filter(|s| matches(s, who) && s.start >= since) {
        match totals.iter_mut().find(|(game, _)| *game == s.game) {
            Some((_, seconds)) => *seconds += s.secs(),
            None => totals.push((s.game.clone(), s.secs())),
        }
    }
    for total in &mut totals {
        total.1 /= 60;
    }
    totals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    totals.truncate(count);
    totals
}

/// Only the sessions that belong to `who`. `None` means everyone, so all sessions.
fn matches(session: &Session, who: Option<&str>) -> bool {
    who.is_none_or(|w| session.member.as_deref() == Some(w))
}

/// Adds a session, split at local midnight so each day's limit sees only its own play.
pub fn record(sessions: &mut Vec<Session>, session: Session, offset: i64) {
    for (start, seconds) in split_by_day(session.start, session.secs(), offset) {
        sessions.push(Session { start, seconds, minutes: 0, ..session.clone() });
    }
    let excess = sessions.len().saturating_sub(MAX_SESSIONS);
    sessions.drain(..excess);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(member: Option<&str>, game: &str, start: i64, minutes: u32) -> Session {
        Session { member: member.map(String::from), game: game.into(), start, seconds: minutes * 60, minutes: 0 }
    }

    #[test]
    fn pins_are_salted_and_checked() {
        let stored = new_pin_hash("1234").unwrap();
        assert!(check_pin(&stored, "1234"));
        assert!(!check_pin(&stored, "4321"));
        assert!(!is_old_hash(&stored));
        assert_ne!(new_pin_hash("1234").unwrap(), stored, "each hash gets its own salt");
        assert!(!check_pin("no-separator", "1234"));
    }

    #[test]
    fn old_sha_pins_still_work() {
        let old = format!("abcd${}", sha_hex("abcd", "1234"));
        assert!(check_pin(&old, "1234"));
        assert!(!check_pin(&old, "1235"));
        assert!(is_old_hash(&old));
    }

    #[test]
    fn play_is_split_at_local_midnight() {
        // 23:30 to 00:45 UTC+1, as UTC seconds: midnight local is 23:00 UTC.
        let day = 20_000 * 86_400;
        let start = day + 22 * 3600 + 30 * 60;
        let parts = split_by_day(start, 75 * 60, 3600);
        assert_eq!(parts, vec![(start, 30 * 60), (day + 23 * 3600, 45 * 60)]);
        assert_eq!(split_by_day(start, 0, 3600), vec![]);
        let mut sessions = vec![];
        record(&mut sessions, Session { seconds: 75 * 60, start, ..Default::default() }, 3600);
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn short_play_still_counts() {
        let s = vec![Session { seconds: 40, ..Default::default() }, Session { seconds: 40, ..Default::default() }];
        assert_eq!(minutes_since(&s, None, 0), 1);
    }

    #[test]
    fn minutes_follow_the_member_and_the_day() {
        let sessions = vec![
            session(Some("kid"), "steam:1", 100, 30),
            session(Some("kid"), "steam:2", 50, 999),
            session(Some("dad"), "steam:1", 200, 45),
        ];
        assert_eq!(minutes_since(&sessions, Some("kid"), 100), 30);
        assert_eq!(minutes_since(&sessions, None, 100), 75);
    }

    #[test]
    fn top_games_sum_then_rank() {
        let sessions = vec![
            session(None, "a", 1, 10),
            session(None, "b", 1, 40),
            session(None, "a", 2, 35),
        ];
        assert_eq!(top_games(&sessions, None, 0, 2), vec![("a".into(), 45), ("b".into(), 40)]);
    }

    #[test]
    fn old_sessions_are_dropped() {
        let mut sessions = vec![];
        for i in 0..(MAX_SESSIONS as i64 + 5) {
            record(&mut sessions, session(None, "x", i * 86_400, 1), 0);
        }
        assert_eq!(sessions.len(), MAX_SESSIONS);
        assert_eq!(sessions[0].start, 5 * 86_400);
    }
}
