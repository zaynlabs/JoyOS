use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use super::run_hidden;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// `None` on PCs without a battery.
    pub battery: Option<u8>,
    pub charging: bool,
    pub wifi: Option<String>,
}

pub fn read() -> Status {
    let mut power = SYSTEM_POWER_STATUS::default();
    let has_power = unsafe { GetSystemPowerStatus(&mut power).is_ok() };
    // 128 = no battery, 255 = unknown.
    let battery = (has_power && power.BatteryFlag & 128 == 0 && power.BatteryLifePercent <= 100)
        .then_some(power.BatteryLifePercent);
    Status { battery, charging: power.ACLineStatus == 1, wifi: wifi_name() }
}

fn wifi_name() -> Option<String> {
    parse_ssid(&run_hidden("netsh", &["wlan", "show", "interfaces"])?)
}

/// `netsh` output is translated, but the `SSID` label is the same everywhere.
fn parse_ssid(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|l| l.starts_with("SSID"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, name)| name.trim().to_owned())
        .filter(|n| !n.is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_ssid_not_bssid() {
        let text = "    Name                   : Wi-Fi\n    State                  : connected\n    SSID                   : Home Net\n    BSSID                  : aa:bb\n";
        assert_eq!(super::parse_ssid(text).as_deref(), Some("Home Net"));
        assert_eq!(super::parse_ssid("There is no wireless interface"), None);
    }
}
