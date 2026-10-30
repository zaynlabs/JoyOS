//! Kiosk setup: JoyOS as the whole desktop, for venues and dedicated machines.
//! Making that change touches the machine-wide shell setting, so JoyOS only
//! writes two scripts. You read them, then run them yourself as administrator.

use std::path::PathBuf;

use anyhow::Result;

use crate::system::{launch, registry, window};

const WINLOGON: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon";

/// Writes `set-shell.ps1` and `restore-shell.ps1`, and returns their folder.
pub fn write_scripts() -> Result<PathBuf> {
    let dir = crate::settings::data_root().ok_or_else(|| anyhow::anyhow!("no data folder"))?.join("kiosk");
    std::fs::create_dir_all(&dir)?;
    let exe = std::env::current_exe()?;
    std::fs::write(dir.join("set-shell.ps1"), set_script(&exe.to_string_lossy()))?;
    std::fs::write(dir.join("restore-shell.ps1"), RESTORE)?;
    Ok(dir)
}

/// Whether Windows starts JoyOS instead of its own desktop.
pub fn we_are_the_shell() -> bool {
    // A per-user shell setting wins over the machine-wide one.
    let shell = registry::read_string(WINLOGON, "Shell").or_else(|| registry::read_local_machine_string(WINLOGON, "Shell"));
    shell.is_some_and(|s| s.to_lowercase().contains("joyos"))
}

/// "Windows desktop": steps aside, or, when JoyOS is the whole desktop, starts the Windows one.
pub fn show_desktop() {
    if we_are_the_shell() {
        // With no desktop running, Explorer starts as the full desktop, taskbar and all.
        launch::open("explorer.exe");
    }
    window::minimize_us();
}

/// Opens the undo script in Notepad, so it can be read before running it.
pub fn show_undo_script() -> bool {
    let Ok(dir) = write_scripts() else { return false };
    launch::open_in("notepad.exe", Some(&format!("\"{}\"", dir.join("restore-shell.ps1").display())), None)
}

/// PowerShell single-quoted text: the only character that needs care is `'` itself.
fn ps_quote(text: &str) -> String {
    text.replace('\'', "''")
}

fn set_script(exe: &str) -> String {
    let exe = ps_quote(exe);
    format!(
        r#"#Requires -RunAsAdministrator
# Makes JoyOS the Windows shell for every user on this PC. It replaces the desktop.
# Undo with restore-shell.ps1. To get the desktop back at any time, hold
# LB + RB + View + Start for 5 seconds (or Ctrl+Alt+Shift+D on a keyboard).
# If JoyOS won't start: press Ctrl+Shift+Esc, choose Run new task, type explorer.exe.
$key = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
Set-ItemProperty -Path $key -Name Shell -Value '"{exe}"'
Write-Host 'JoyOS is now the shell. Restart the PC to take effect.'
"#
    )
}

const RESTORE: &str = r#"#Requires -RunAsAdministrator
# Puts the normal Windows desktop back as the shell.
$key = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
Set-ItemProperty -Path $key -Name Shell -Value 'explorer.exe'
Write-Host 'The Windows desktop is back. Restart the PC to take effect.'
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_names_this_exe_and_the_recovery_steps() {
        let script = set_script(r"C:\Games\JoyOS\joyos.exe");
        assert!(script.contains(r#"'"C:\Games\JoyOS\joyos.exe"'"#));
        assert!(script.contains("explorer.exe"));
        assert!(RESTORE.contains("explorer.exe"));
    }

    #[test]
    fn quotes_in_the_path_stay_inside_the_string() {
        let script = set_script(r"C:\Users\O'Brien\joyos.exe");
        assert!(script.contains(r#"'"C:\Users\O''Brien\joyos.exe"'"#));
    }
}
