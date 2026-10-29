//! Synthetic mouse and keyboard input, for using the controller as a mouse.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSE_EVENT_FLAGS, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput, VIRTUAL_KEY, VK_LWIN, VK_SNAPSHOT,
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
