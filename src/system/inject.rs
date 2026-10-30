//! Synthetic mouse and keyboard input, for using the controller as a mouse.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE,
    MOUSE_EVENT_FLAGS, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput, VIRTUAL_KEY, VK_G, VK_H, VK_LWIN, VK_MENU,
    VK_SNAPSHOT,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn mouse(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, data: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx, dy, mouseData: data as _, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn keyboard(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    }
}

pub fn move_by(dx: i32, dy: i32) {
    if dx != 0 || dy != 0 {
        send(&[mouse(MOUSEEVENTF_MOVE, dx, dy, 0)]);
    }
}

pub fn button(which: MouseButton, down: bool) {
    let flags = match (which, down) {
        (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
        (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
        (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
        (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
    };
    send(&[mouse(flags, 0, 0, 0)]);
}

/// Positive scrolls up / right. 120 is one notch.
pub fn scroll(vertical: i32, horizontal: i32) {
    if vertical != 0 {
        send(&[mouse(MOUSEEVENTF_WHEEL, 0, 0, vertical)]);
    }
    if horizontal != 0 {
        send(&[mouse(MOUSEEVENTF_HWHEEL, 0, 0, horizontal)]);
    }
}

pub fn key(vk: VIRTUAL_KEY, down: bool) {
    let flags = if down { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP };
    send(&[keyboard(vk, flags)]);
}

/// Types text into whatever has focus, whatever the user's keyboard layout.
pub fn text(text: &str) {
    send(&unicode(text));
}

/// Each UTF-16 unit goes in as its own down/up pair; Windows rebuilds surrogate pairs.
fn unicode(text: &str) -> Vec<INPUT> {
    let key = |unit: u16, flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0), wScan: unit, dwFlags: flags, time: 0, dwExtraInfo: 0 },
        },
    };
    text.encode_utf16()
        .flat_map(|u| [key(u, KEYEVENTF_UNICODE), key(u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)])
        .collect()
}

/// Presses the keys in order, then releases them in reverse.
pub fn chord(keys: &[VIRTUAL_KEY]) {
    let mut inputs: Vec<INPUT> = keys.iter().map(|&k| keyboard(k, KEYBD_EVENT_FLAGS(0))).collect();
    inputs.extend(keys.iter().rev().map(|&k| keyboard(k, KEYEVENTF_KEYUP)));
    send(&inputs);
}

/// Win + Print Screen: Windows saves the shot to Pictures\Screenshots.
pub fn screenshot() {
    chord(&[VK_LWIN, VK_SNAPSHOT]);
}

/// Win + Alt + G: the Game Bar saves the last 30 seconds of play.
pub fn game_clip() {
    chord(&[VK_LWIN, VK_MENU, VK_G]);
}

/// Win + H: Windows dictation, typing what you say into the focused text box.
pub fn dictate() {
    chord(&[VK_LWIN, VK_H]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_becomes_unicode_key_pairs() {
        let inputs = unicode("é😀");
        // One unit for é, a surrogate pair for the emoji, each pressed and released.
        assert_eq!(inputs.len(), 6);
        let scans: Vec<u16> = inputs.iter().map(|i| unsafe { i.Anonymous.ki.wScan }).collect();
        assert_eq!(scans, [0xe9, 0xe9, 0xd83d, 0xd83d, 0xde00, 0xde00]);
        let up = unsafe { inputs[1].Anonymous.ki.dwFlags };
        assert_eq!(up, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
    }
}
