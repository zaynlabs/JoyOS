//! The keyboard that floats over other apps. It lives in its own small window
//! that never takes focus, so what it types goes to the app underneath, and the
//! controller drives it directly: there's no mouse to aim.

use std::cell::Cell;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};

use dioxus::desktop::tao::window::WindowId;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use dioxus::prelude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures_util::StreamExt;
use serde_json::json;
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_BACK, VK_LEFT, VK_RETURN, VK_RIGHT};

use crate::input::{Dir, Family, Pad, SHARED};
use crate::system::{inject, window};
use crate::ui::keyboard::layout::{self, DIGITS, LATIN, SYMBOLS};
use crate::ui::{Glyph, PadFamily};
use crate::{i18n, t, words};

#[derive(Clone, Debug)]
pub enum Msg {
    /// `look` is the same attribute script the main window uses, so theme, contrast and text size match.
    Show { look: String, sounds: bool, reduced: bool, family: Family, words: Vec<String> },
    Hide,
    Move(Dir),
    Press(Pad),
}

static TX: OnceLock<UnboundedSender<Msg>> = OnceLock::new();
static RX: Mutex<Option<UnboundedReceiver<Msg>>> = Mutex::new(None);
static HWND: AtomicIsize = AtomicIsize::new(0);

thread_local! {
    static ID: Cell<Option<WindowId>> = const { Cell::new(None) };
}

pub fn send(msg: Msg) {
    if let Some(tx) = TX.get() {
        let _ = tx.unbounded_send(msg);
    }
}

pub fn is_open() -> bool {
    SHARED.typing.load(Ordering::Relaxed)
}

/// The overlay's window, so JoyOS can close it when the main window closes.
pub fn window_id() -> Option<WindowId> {
    ID.with(Cell::get)
}

/// Builds the window once, hidden; showing it later is instant.
pub fn create() {
    let (tx, rx) = unbounded();
    if TX.set(tx).is_err() {
        return;
    }
    *RX.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);

    use dioxus::desktop::tao::dpi::PhysicalPosition;
    use dioxus::desktop::tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
    // Parked off screen until the first Show: Dioxus makes every window visible once it loads.
    let builder = WindowBuilder::new()
        .with_title("JoyOS keyboard")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_focused(false)
        .with_skip_taskbar(true)
        .with_position(PhysicalPosition::new(-32000, -32000))
        .with_inner_size(LogicalSize::new(1200.0, 520.0));
    let cfg: Config = crate::config(builder, (0, 0, 0, 0), "<script>document.documentElement.dataset.window='osk'</script>")
        .with_on_window(|w, _| {
            let hwnd = w.hwnd();
            HWND.store(hwnd, Ordering::Relaxed);
            ID.with(|id| id.set(Some(w.id())));
            // Before the first show, so it can never steal focus.
            window::make_overlay(hwnd);
        });
    let _ = dioxus::desktop::window().new_window(VirtualDom::new(Osk), cfg);
}

fn hwnd() -> isize {
    HWND.load(Ordering::Relaxed)
}

fn tap(vk: VIRTUAL_KEY) {
    inject::key(vk, true);
    inject::key(vk, false);
}

/// Keyboard state that every key reads.
#[derive(Clone, Copy)]
struct Board {
    shift: Signal<bool>,
    symbols: Signal<bool>,
    latin: Signal<bool>,
    /// The word being typed into the app underneath, for suggestions.
    fragment: Signal<String>,
    /// Words from JoyOS's history, passed in when the keyboard opens.
    words: Signal<Vec<String>>,
    bridge: document::Eval,
}

impl Board {
    fn send(&self, msg: serde_json::Value) {
        let _ = self.bridge.send(msg);
    }

    fn sound(&self, kind: &str) {
        self.send(json!({ "t": "sound", "s": kind }));
    }

    fn close(&self) {
        SHARED.typing.store(false, Ordering::Relaxed);
        window::hide(hwnd());
        self.sound("back");
    }

    fn type_char(mut self, ch: char) {
        let text = if (self.shift)() { layout::upper(ch, i18n::current().code) } else { ch.to_string() };
        inject::text(&text);
        self.fragment.write().push_str(&text);
        if (self.shift)() {
            self.shift.set(false);
        }
    }

    fn space(mut self) {
        inject::text(" ");
        self.fragment.set(String::new());
    }

    fn delete(mut self) {
        tap(VK_BACK);
        self.fragment.write().pop();
        self.sound("back");
    }

    /// Replaces the word being typed with `word`, the way tapping a suggestion does.
    fn complete(mut self, word: &str) {
        let typed = self.fragment.read().chars().count();
        for _ in 0..typed {
            tap(VK_BACK);
        }
        inject::text(&format!("{word} "));
        self.fragment.set(String::new());
        self.sound("select");
    }

    fn press(mut self, pad: Pad) {
        match pad {
            Pad::A => self.send(json!({ "t": "activate" })),
            Pad::B | Pad::Start => self.close(),
            Pad::X => self.delete(),
            Pad::Y => {
                self.space();
                self.sound("tick");
            }
            Pad::Lt => {
                self.shift.set(!(self.shift)());
                self.sound("tick");
            }
            Pad::Rt => {
                tap(VK_RETURN);
                self.fragment.set(String::new());
                self.sound("select");
            }
            // Moving the cursor leaves the word behind, so a suggestion mustn't erase it.
            Pad::Lb => {
                tap(VK_LEFT);
                self.fragment.set(String::new());
            }
            Pad::Rb => {
                tap(VK_RIGHT);
                self.fragment.set(String::new());
            }
            Pad::Select => {
                self.symbols.set(!(self.symbols)());
                self.sound("tick");
            }
        }
    }
}

#[component]
pub fn Osk() -> Element {
    let bridge = use_hook(|| document::eval(crate::app::NAV_JS));
    let board = use_context_provider(|| Board {
        shift: Signal::new(false),
        symbols: Signal::new(false),
        latin: Signal::new(false),
        fragment: Signal::new(String::new()),
        words: Signal::new(vec![]),
        bridge,
    });
    let family = use_context_provider(|| PadFamily(Signal::new(Family::default())));
    // Bumped on every show so the keys remount and focus starts on the first letter.
    let mut shown = use_signal(|| 0u64);

    // Dioxus shows every window once its page loads; put this one away until it's asked for.
    use_future(move || async move {
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if !is_open() && window::is_visible(hwnd()) {
                window::hide(hwnd());
                break;
            }
        }
    });

    use_future(move || async move {
        let rx = RX.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some(mut rx) = rx else { return };
        let mut family = family.0;
        let (mut shift, mut symbols) = (board.shift, board.symbols);
        let mut words_signal = board.words;
        while let Some(msg) = rx.next().await {
            match msg {
                Msg::Show { look, sounds, reduced, family: f, words: history } => {
                    let lang = i18n::current();
                    document::eval(&look);
                    board.send(json!({ "t": "prefs", "sounds": sounds, "reduced": reduced, "lang": lang.code, "rtl": lang.rtl }));
                    family.set(f);
                    shift.set(false);
                    symbols.set(false);
                    words_signal.set(history);
                    let mut fragment = board.fragment;
                    fragment.set(String::new());
                    let next = *shown.peek() + 1;
                    shown.set(next);
                    SHARED.typing.store(true, Ordering::Relaxed);
                    window::show_overlay(hwnd());
                    board.sound("open");
                }
                Msg::Hide => {
                    if is_open() {
                        board.close();
                    }
                }
                Msg::Move(d) => {
                    let d = match d {
                        Dir::Up => "up",
                        Dir::Down => "down",
                        Dir::Left => "left",
                        Dir::Right => "right",
                    };
                    board.send(json!({ "t": "move", "d": d }));
                }
                Msg::Press(p) => board.press(p),
            }
        }
    });

    let code = i18n::current().code;
    let shift = (board.shift)();
    let symbols = (board.symbols)();
    let latin = (board.latin)();
    let rows: &[&str] = if symbols {
        SYMBOLS
    } else if layout::has_own_script(code) && latin {
        LATIN
    } else {
        layout::native_rows(code).unwrap_or(LATIN)
    };
    let page_label = if symbols { layout::script_sample(code) } else { "?123" };
    let script_label = if latin { layout::script_sample(code) } else { "ABC" };
    let lang = i18n::current().code;
    let dir = if i18n::current().rtl { "rtl" } else { "ltr" };
    let suggestions = words::suggest(&(board.words)(), &(board.fragment)());

    rsx! {
        div { class: "osk glass", "data-main": "osk", lang, dir,
            if !suggestions.is_empty() {
                div { class: "suggest", "data-row": "suggest",
                    for word in suggestions {
                        button { class: "chip", "data-nav": "", "data-sound": "tick", onclick: move |_| board.complete(&word), "{word}" }
                    }
                }
            }
            for n in [shown()] {
                div { key: "{n}", class: "keys",
                    div { class: "krow", "data-row": "digits",
                        for ch in DIGITS.chars() {
                            Key { ch }
                        }
                    }
                    for (r, row) in rows.iter().enumerate() {
                        div { key: "{r}-{symbols}-{latin}", class: "krow", "data-row": "r{r}",
                            for (i, ch) in row.chars().enumerate() {
                                Key { ch, shift, autofocus: r == 0 && i == 0 }
                            }
                        }
                    }
                    div { class: "krow", "data-row": "actions",
                        button {
                            class: if shift { "key wide on" } else { "key wide" },
                            "data-nav": "",
                            "data-sound": "tick",
                            onclick: move |_| board.press(Pad::Lt),
                            {t!("key-shift")}
                        }
                        button { class: "key wide", "data-nav": "", "data-sound": "tick",
                            onclick: move |_| board.press(Pad::Select),
                            "{page_label}"
                        }
                        if layout::has_own_script(code) && !symbols {
                            button { class: "key wide", "data-nav": "", "data-sound": "tick",
                                onclick: move |_| {
                                    let mut latin = board.latin;
                                    latin.set(!latin());
                                },
                                "{script_label}"
                            }
                        }
                        button { class: "key space", "data-nav": "", "data-sound": "tick", onclick: move |_| board.space(), {t!("key-space")} }
                        button { class: "key wide", "data-nav": "", "data-sound": "back", onclick: move |_| board.delete(), {t!("key-delete")} }
                        button { class: "key wide", "data-nav": "", onclick: move |_| {
                            tap(VK_RETURN);
                            let mut fragment = board.fragment;
                            fragment.set(String::new());
                        }, {t!("key-enter")} }
                        button { class: "key wide", "data-nav": "", "data-sound": "select", onclick: move |_| inject::dictate(), {t!("key-voice")} }
                        button { class: "key wide on", "data-nav": "", "data-sound": "back", onclick: move |_| board.close(), {t!("action-done")} }
                    }
                }
            }
            footer { class: "osk-hints",
                span { class: "hint", Glyph { pad: Pad::X } {t!("key-delete")} }
                span { class: "hint", Glyph { pad: Pad::Y } {t!("key-space")} }
                span { class: "hint", Glyph { pad: Pad::Lt } {t!("key-shift")} }
                span { class: "hint", Glyph { pad: Pad::Rt } {t!("key-enter")} }
                span { class: "hint", Glyph { pad: Pad::Lb } Glyph { pad: Pad::Rb } {t!("osk-caret")} }
                span { class: "hint", Glyph { pad: Pad::B } {t!("action-done")} }
            }
        }
    }
}

#[component]
fn Key(ch: char, #[props(default)] shift: bool, #[props(default)] autofocus: bool) -> Element {
    let board = use_context::<Board>();
    let label = if shift { layout::upper(ch, i18n::current().code) } else { ch.to_string() };
    rsx! {
        button {
            class: "key",
            "data-nav": "",
            "data-sound": "tick",
            "data-autofocus": autofocus.then_some("true"),
            onclick: move |_| board.type_char(ch),
            "{label}"
        }
    }
}
