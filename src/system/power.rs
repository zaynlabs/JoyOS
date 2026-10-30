use windows::Win32::System::Power::SetSuspendState;

use super::run_hidden;
use crate::settings::PowerMode;

pub fn sleep() {
    unsafe {
        SetSuspendState(false, false, false);
    }
}

pub fn restart() {
    run_hidden("shutdown", &["/r", "/t", "0"]);
}

pub fn turn_off() {
    run_hidden("shutdown", &["/s", "/hybrid", "/t", "0"]);
}

// Windows' built-in plans. They're the same on every PC, and their GUIDs are fixed.
const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
const PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
const SAVER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a";

/// Switches to the plan for `mode` and returns the plan that was active, so it can be restored.
pub fn set_mode(mode: PowerMode) -> Option<String> {
    let before = active_plan();
    let guid = match mode {
        PowerMode::Balanced => BALANCED,
        PowerMode::Performance => PERFORMANCE,
        PowerMode::Saver => SAVER,
    };
    run_hidden("powercfg", &["/setactive", guid]);
    before
}

pub fn restore_plan(guid: &str) {
    run_hidden("powercfg", &["/setactive", guid]);
}

/// `powercfg` prints the GUID in every language, so find it by shape, not by label.
fn active_plan() -> Option<String> {
    let out = run_hidden("powercfg", &["/getactivescheme"])?;
    find_guid(&out)
}

fn find_guid(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|w| w.len() == 36 && w.matches('-').count() == 4 && w.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_plan_guid_in_any_language() {
        let text = "Stromschema-GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Ausgeglichen)\r\n";
        assert_eq!(find_guid(text).as_deref(), Some(BALANCED));
        assert_eq!(find_guid("nothing here"), None);
    }
}
