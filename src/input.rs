//! The controller thread. It turns raw gamepad state into what the shell needs:
//! menu moves with console-style repeat when JoyOS is in front, and mouse and
//! keyboard input when another app is. While the on-screen keyboard is up over
//! another app, the controller drives the keyboard instead of the mouse.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_BROWSER_BACK, VK_BROWSER_FORWARD, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_UP,
};

use crate::system::{inject, textfield, window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pad {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Lt,
    Rt,
    Start,
    Select,
}

/// Which button labels to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Family {
    #[default]
    Xbox,
    PlayStation,
    Nintendo,
}

impl Family {
    fn from_name(name: &str) -> Self {
        let n = name.to_lowercase();
        if ["dualsense", "dualshock", "playstation", "ps4", "ps5", "wireless controller"].iter().any(|k| n.contains(k)) {
            Family::PlayStation
        } else if ["nintendo", "pro controller", "joy-con", "switch"].iter().any(|k| n.contains(k)) {
            Family::Nintendo
        } else {
            Family::Xbox
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    Move(Dir),
    Press(Pad),
    /// Right stick, for the parallax tilt on the focused tile. -1.0 – 1.0.
    Tilt(f32, f32),
    /// Open the quick menu and bring JoyOS to the front, from anywhere.
    Summon,
    /// Show the on-screen keyboard over the app in front. `toggle` hides it if it's
    /// already up; picking a text box only ever shows it.
    Keyboard { toggle: bool },
    Connected(String, Family),
    Disconnected,
}

/// Settings the UI changes and the input thread reads.
pub struct Shared {
    pub mouse_mode: AtomicBool,
    pub swap_ab: AtomicBool,
    pub mouse_speed: AtomicU8,
    pub rumble: AtomicBool,
    /// JoyOS's own keyboard is up over another app.
    pub typing: AtomicBool,
    rumble_now: AtomicBool,
}

pub static SHARED: Shared = Shared {
    mouse_mode: AtomicBool::new(false),
    swap_ab: AtomicBool::new(false),
    mouse_speed: AtomicU8::new(5),
    rumble: AtomicBool::new(true),
    typing: AtomicBool::new(false),
    rumble_now: AtomicBool::new(false),
};

/// A short, soft buzz (e.g. when a game starts).
pub fn rumble() {
    SHARED.rumble_now.store(true, Ordering::Relaxed);
}

pub fn start() -> UnboundedReceiver<InputEvent> {
    let (tx, rx) = unbounded();
    std::thread::Builder::new()
        .name("controller".into())
        .spawn(move || run(tx))
        .expect("spawn controller thread");
    rx
}

const TICK: Duration = Duration::from_millis(8);
const FIRST_REPEAT: Duration = Duration::from_millis(380);
const STICK_ON: f32 = 0.55;
const STICK_DEAD: f32 = 0.14;

/// Console-style held-direction repeat that speeds up the longer you hold.
#[derive(Default)]
struct Repeat {
    held: Option<Dir>,
    next: Option<Instant>,
    count: u32,
}

impl Repeat {
    fn update(&mut self, dir: Option<Dir>, now: Instant) -> Option<Dir> {
        if dir != self.held {
            self.held = dir;
            self.count = 0;
            self.next = dir.map(|_| now + FIRST_REPEAT);
            return dir;
        }
        let (dir, next) = (dir?, self.next?);
        if now < next {
            return None;
        }
        self.count += 1;
        let gap = if self.count > 6 { 55 } else { 95 };
        self.next = Some(now + Duration::from_millis(gap));
        Some(dir)
    }
}

fn stick_dir(x: f32, y: f32) -> Option<Dir> {
    if x.abs().max(y.abs()) < STICK_ON {
        return None;
    }
    Some(if x.abs() > y.abs() {
        if x > 0.0 { Dir::Right } else { Dir::Left }
    } else if y > 0.0 {
        Dir::Up
    } else {
        Dir::Down
    })
}

/// Deadzone, then a curve so small pushes give fine control.
fn shape(v: f32) -> f32 {
    let a = ((v.abs() - STICK_DEAD) / (1.0 - STICK_DEAD)).clamp(0.0, 1.0);
    a.powf(2.2) * v.signum()
}

fn map_button(b: Button, swap: bool) -> Option<Pad> {
    Some(match b {
        Button::South if swap => Pad::B,
        Button::East if swap => Pad::A,
        Button::South => Pad::A,
        Button::East => Pad::B,
        Button::West => Pad::X,
        Button::North => Pad::Y,
        Button::LeftTrigger => Pad::Lb,
        Button::RightTrigger => Pad::Rb,
        Button::LeftTrigger2 => Pad::Lt,
        Button::RightTrigger2 => Pad::Rt,
        Button::Start => Pad::Start,
        Button::Select => Pad::Select,
        _ => return None,
    })
}

fn dpad_key(b: Button) -> Option<VIRTUAL_KEY> {
    Some(match b {
        Button::DPadUp => VK_UP,
        Button::DPadDown => VK_DOWN,
        Button::DPadLeft => VK_LEFT,
        Button::DPadRight => VK_RIGHT,
        _ => return None,
    })
}

fn run(tx: UnboundedSender<InputEvent>) {
    let mut gilrs = match Gilrs::new() {
        Ok(g) => g,
        Err(gilrs::Error::NotImplemented(g)) => g,
        Err(e) => {
            tracing::warn!("no controller support: {e}");
            return;
        }
    };
    let send = |e: InputEvent| {
        let _ = tx.unbounded_send(e);
    };

    let mut active: Option<GamepadId> = gilrs.gamepads().next().map(|(id, _)| id);
    if let Some(id) = active {
        let name = gilrs.gamepad(id).name().to_owned();
        send(InputEvent::Connected(name.clone(), Family::from_name(&name)));
    }

    let mut repeat = Repeat::default();
    let mut last_tilt = (0.0f32, 0.0f32);
    let mut last_tilt_at = Instant::now();
    let mut carry = (0.0f32, 0.0f32, 0.0f32);
    let mut last = Instant::now();
    // What a button did when pressed, so release undoes the same thing even if
    // focus moved in between.
    let mut mouse_held: Vec<(Button, inject::MouseButton)> = vec![];
    let mut keys_held: Vec<(Button, VIRTUAL_KEY)> = vec![];

    loop {
        let in_front = window::we_are_in_front();
        let typing = !in_front && SHARED.typing.load(Ordering::Relaxed);
        let mouse_mode = !in_front && !typing && SHARED.mouse_mode.load(Ordering::Relaxed);
        let swap = SHARED.swap_ab.load(Ordering::Relaxed);

        while let Some(ev) = gilrs.next_event() {
            match ev.event {
                EventType::Connected => {
                    active = Some(ev.id);
                    let name = gilrs.gamepad(ev.id).name().to_owned();
                    send(InputEvent::Connected(name.clone(), Family::from_name(&name)));
                }
                EventType::Disconnected => {
                    active = gilrs.gamepads().map(|(id, _)| id).find(|&id| id != ev.id);
                    match active {
                        Some(id) => {
                            let name = gilrs.gamepad(id).name().to_owned();
                            send(InputEvent::Connected(name.clone(), Family::from_name(&name)));
                        }
                        None => send(InputEvent::Disconnected),
                    }
                }
                EventType::ButtonPressed(button, _) => {
                    active = Some(ev.id);
                    let pad = gilrs.gamepad(ev.id);
                    let chord = matches!(button, Button::Start | Button::Select)
                        && pad.is_pressed(Button::Start)
                        && pad.is_pressed(Button::Select);
                    if button == Button::Mode || chord || (mouse_mode && button == Button::Start) {
                        send(InputEvent::Summon);
                        continue;
                    }
                    if in_front || typing {
                        if let Some(p) = map_button(button, swap) {
                            send(InputEvent::Press(p));
                        }
                    } else if mouse_mode {
                        let click = match button {
                            Button::South => Some(inject::MouseButton::Left),
                            Button::East => Some(inject::MouseButton::Right),
                            Button::West => Some(inject::MouseButton::Middle),
                            _ => None,
                        };
                        let key = dpad_key(button).or(match button {
                            Button::Select => Some(VK_ESCAPE),
                            Button::RightTrigger2 => Some(VK_RETURN),
                            Button::LeftTrigger => Some(VK_BROWSER_BACK),
                            Button::RightTrigger => Some(VK_BROWSER_FORWARD),
                            _ => None,
                        });
                        if let Some(c) = click {
                            inject::button(c, true);
                            mouse_held.push((button, c));
                        } else if let Some(k) = key {
                            inject::key(k, true);
                            keys_held.push((button, k));
                        } else if button == Button::North {
                            send(InputEvent::Keyboard { toggle: true });
                        }
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    let mut clicked = false;
                    mouse_held.retain(|&(b, c)| {
                        if b == button {
                            inject::button(c, false);
                            clicked |= c == inject::MouseButton::Left;
                        }
                        b != button
                    });
                    if clicked {
                        offer_keyboard(tx.clone());
                    }
                    keys_held.retain(|&(b, k)| {
                        if b == button {
                            inject::key(k, false);
                        }
                        b != button
                    });
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32().min(0.05);
        last = now;

        if let Some(pad) = active.and_then(|id| gilrs.connected_gamepad(id)) {
            let (lx, ly) = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            let (rx, ry) = (pad.value(Axis::RightStickX), pad.value(Axis::RightStickY));

            if in_front || typing {
                let dpad = [
                    (Button::DPadUp, Dir::Up),
                    (Button::DPadDown, Dir::Down),
                    (Button::DPadLeft, Dir::Left),
                    (Button::DPadRight, Dir::Right),
                ]
                .into_iter()
                .find(|&(b, _)| pad.is_pressed(b))
                .map(|(_, d)| d);
                if let Some(dir) = repeat.update(dpad.or_else(|| stick_dir(lx, ly)), now) {
                    send(InputEvent::Move(dir));
                }

                let tilt = if typing { (0.0, 0.0) } else { (shape(rx), shape(ry)) };
                let changed = (tilt.0 - last_tilt.0).abs() + (tilt.1 - last_tilt.1).abs() > 0.03;
                let settled = tilt == (0.0, 0.0) && last_tilt != (0.0, 0.0);
                if (changed && now.duration_since(last_tilt_at) > Duration::from_millis(30)) || settled {
                    send(InputEvent::Tilt(tilt.0, tilt.1));
                    last_tilt = tilt;
                    last_tilt_at = now;
                }
            } else {
                repeat.update(None, now);
                if mouse_mode {
                    let speed = SHARED.mouse_speed.load(Ordering::Relaxed).clamp(1, 10) as f32;
                    let px_per_sec = 500.0 + speed * 190.0;
                    // Holding the left trigger slows the cursor for precise clicks.
                    let precise = if pad.is_pressed(Button::LeftTrigger2) { 0.3 } else { 1.0 };
                    carry.0 += shape(lx) * px_per_sec * precise * dt;
                    carry.1 -= shape(ly) * px_per_sec * precise * dt;
                    carry.2 += shape(ry) * 2400.0 * dt;
                    let (mx, my) = (carry.0.trunc() as i32, carry.1.trunc() as i32);
                    carry.0 -= mx as f32;
                    carry.1 -= my as f32;
                    inject::move_by(mx, my);
                    if carry.2.abs() >= 20.0 {
                        inject::scroll(carry.2 as i32, 0);
                        carry.2 = 0.0;
                    }
                    let h = shape(rx) * 1600.0 * dt;
                    if h.abs() >= 8.0 {
                        inject::scroll(0, h as i32);
                    }
                }
            }
        }

        if SHARED.rumble_now.swap(false, Ordering::Relaxed)
            && SHARED.rumble.load(Ordering::Relaxed)
            && let Some(id) = active
            && gilrs.gamepad(id).is_ff_supported()
        {
            let effect = EffectBuilder::new()
                .add_effect(BaseEffect {
                    kind: BaseEffectType::Weak { magnitude: 26_000 },
                    scheduling: Replay { play_for: Ticks::from_ms(90), ..Default::default() },
                    ..Default::default()
                })
                .gamepads(&[id])
                .finish(&mut gilrs);
            if let Ok(effect) = effect {
                let _ = effect.play();
                // Effects stop when dropped; this one is short, so let it finish.
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(120));
                    drop(effect);
                });
            }
        }

        std::thread::sleep(TICK);
    }
}

/// After a click, opens the keyboard if the click landed in a text box. Focus
/// takes a moment to move, and UI Automation can be slow, so this gets its own thread.
fn offer_keyboard(tx: UnboundedSender<InputEvent>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        if textfield::focused() {
            let _ = tx.unbounded_send(InputEvent::Keyboard { toggle: false });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_fires_once_then_waits_then_speeds_up() {
        let mut r = Repeat::default();
        let t0 = Instant::now();
        assert_eq!(r.update(Some(Dir::Down), t0), Some(Dir::Down));
        assert_eq!(r.update(Some(Dir::Down), t0 + Duration::from_millis(100)), None);
        assert_eq!(r.update(Some(Dir::Down), t0 + FIRST_REPEAT), Some(Dir::Down));
        assert_eq!(r.update(None, t0 + FIRST_REPEAT), None);
        assert_eq!(r.update(Some(Dir::Left), t0 + FIRST_REPEAT), Some(Dir::Left));
    }

    #[test]
    fn stick_picks_the_stronger_axis() {
        assert_eq!(stick_dir(0.9, 0.3), Some(Dir::Right));
        assert_eq!(stick_dir(-0.2, -0.8), Some(Dir::Down));
        assert_eq!(stick_dir(0.3, 0.3), None);
        assert_eq!(shape(0.1), 0.0);
        assert!((shape(-1.0) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn swap_ab_swaps() {
        assert_eq!(map_button(Button::South, false), Some(Pad::A));
        assert_eq!(map_button(Button::South, true), Some(Pad::B));
        assert_eq!(Family::from_name("DualSense Wireless Controller"), Family::PlayStation);
        assert_eq!(Family::from_name("Xbox Controller"), Family::Xbox);
    }
}
