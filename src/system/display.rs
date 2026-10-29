//! Screen brightness. Only built-in screens (laptops, handhelds) support this;
//! on desktops `brightness()` returns `None` and the slider hides itself.

use super::run_hidden;

fn powershell(script: &str) -> Option<String> {
    run_hidden("powershell", &["-NoProfile", "-NonInteractive", "-Command", script])
}

/// 0 – 100. Slow (spawns PowerShell); call off the UI thread.
pub fn brightness() -> Option<u8> {
    powershell("(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction Stop).CurrentBrightness")?
        .trim()
        .lines()
        .next()?
        .parse()
        .ok()
}

pub fn set_brightness(level: u8) {
    let script = format!(
        "Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | \
         Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{Timeout=1;Brightness={}}}",
        level.min(100)
    );
    powershell(&script);
}
