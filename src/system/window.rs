//! Keeping track of which window is in front, and getting JoyOS back on top.

use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW, GetWindowTextW,
    HWND_TOPMOST, IsIconic, IsWindowVisible, PostMessageW, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_CLOSE, WS_EX_NOACTIVATE,
    WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

static OURS: AtomicIsize = AtomicIsize::new(0);

pub fn set_ours(hwnd: isize) {
    OURS.store(hwnd, Ordering::Relaxed);
}

fn ours() -> Option<HWND> {
    match OURS.load(Ordering::Relaxed) {
        0 => None,
        h => Some(HWND(h as _)),
    }
}

/// True when JoyOS has the user's attention (or before our window exists).
pub fn we_are_in_front() -> bool {
    match ours() {
        Some(us) => (unsafe { GetForegroundWindow() }) == us,
        None => true,
    }
}

/// A window the user was using before they called JoyOS up.
#[derive(Clone, Debug, PartialEq)]
pub struct Outside {
    hwnd: isize,
    pub title: String,
}

/// The window in front, if it's an app or game and not JoyOS or the desktop.
pub fn front_outside() -> Option<Outside> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() || Some(hwnd) == ours() || hwnd == GetShellWindow() {
            return None;
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return None;
        }
        let mut title = [0u16; 256];
        let n = GetWindowTextW(hwnd, &mut title) as usize;
        let short = app_name(&String::from_utf16_lossy(&title[..n]));
        (!short.is_empty()).then_some(Outside { hwnd: hwnd.0 as isize, title: short })
    }
}

/// Most windows put the app name last: "Inbox — Mail", "Song - Spotify".
fn app_name(title: &str) -> String {
    let title = title.trim();
    for sep in [" — ", " – ", " - ", " | "] {
        if let Some((_, last)) = title.rsplit_once(sep)
            && !last.trim().is_empty()
        {
            return last.trim().to_owned();
        }
    }
    title.to_owned()
}

/// Windows blocks background apps from stealing focus. A synthetic Alt tap is the
/// accepted way around it for shells and launchers.
fn force_front(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        // Already in front: the Alt tap would land on this very window and open its
        // menu mode, which stalls the page until another key is pressed.
        if GetForegroundWindow() == hwnd {
            return;
        }
        super::inject::key(VK_MENU, true);
        super::inject::key(VK_MENU, false);
        let _ = SetForegroundWindow(hwnd);
    }
}

pub fn bring_us_forward() {
    if let Some(us) = ours() {
        force_front(us);
    }
}

pub fn minimize_us() {
    if let Some(us) = ours() {
        unsafe {
            let _ = ShowWindow(us, SW_MINIMIZE);
        }
    }
}

pub fn focus(target: &Outside) {
    force_front(HWND(target.hwnd as _));
}

pub fn close(target: &Outside) {
    unsafe {
        let _ = PostMessageW(Some(HWND(target.hwnd as _)), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

// ----- the keyboard overlay -----

/// A window that floats over everything and never takes focus, so keys it
/// sends land in the app the user is typing into.
pub fn make_overlay(hwnd: isize) {
    let hwnd = HWND(hwnd as _);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let extra = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize;
        // App windows show in the taskbar and Alt+Tab, which this mustn't.
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (style | extra) & !(WS_EX_APPWINDOW.0 as isize));
        let round = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, (&raw const round).cast(), 4);
        // Acrylic: the app behind blurs through the page's transparent background.
        let acrylic = DWMSBT_TRANSIENTWINDOW;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, (&raw const acrylic).cast(), 4);
    }
}

/// Shows the overlay along the bottom of the screen the front app is on.
pub fn show_overlay(hwnd: isize) {
    unsafe {
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return;
        }
        let work = info.rcWork;
        let (w, h) = (work.right - work.left, work.bottom - work.top);
        let (ow, oh) = (w * 66 / 100, h * 44 / 100);
        let x = work.left + (w - ow) / 2;
        let y = work.bottom - oh - h * 2 / 100;
        let _ = SetWindowPos(HWND(hwnd as _), Some(HWND_TOPMOST), x, y, ow, oh, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

pub fn is_visible(hwnd: isize) -> bool {
    unsafe { IsWindowVisible(HWND(hwnd as _)).as_bool() }
}

pub fn hide(hwnd: isize) {
    unsafe {
        let _ = ShowWindow(HWND(hwnd as _), SW_HIDE);
    }
}

#[cfg(test)]
mod tests {
    use super::app_name;

    #[test]
    fn keeps_the_app_part_of_a_title() {
        assert_eq!(app_name("Never Gonna Give You Up - YouTube — Mozilla Firefox"), "Mozilla Firefox");
        assert_eq!(app_name("Hades II"), "Hades II");
        assert_eq!(app_name("  "), "");
    }
}
