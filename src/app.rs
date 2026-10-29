//! Shell state and the root component. Controller events, JS messages and UI
//! actions all funnel through `Shell`, so behaviour is the same however you drive it.

use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt;
use serde_json::{Value, json};

use crate::input::{Dir, Family, InputEvent, Pad, SHARED};
use crate::library::{Library, Source, Title};
use crate::settings::{Keyboard, Settings, Theme};
use crate::system::{self, off_thread, status::Status, window::Outside};
use crate::{i18n, osk, t, ui};

pub const NAV_JS: &str = include_str!("../assets/js/nav.js");

static INPUT: Mutex<Option<UnboundedReceiver<InputEvent>>> = Mutex::new(None);

pub fn hand_over_input(rx: UnboundedReceiver<InputEvent>) {
    *INPUT.lock().unwrap() = Some(rx);
}

pub fn apply_to_input(s: &Settings) {
    SHARED.swap_ab.store(s.swap_ab, Ordering::Relaxed);
    SHARED.mouse_speed.store(s.mouse_speed, Ordering::Relaxed);
    SHARED.rumble.store(s.rumble, Ordering::Relaxed);
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Home,
    Games,
    Apps,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Home, Tab::Games, Tab::Apps, Tab::Settings];

    pub fn label(self) -> String {
        match self {
            Tab::Home => t!("tab-home"),
            Tab::Games => t!("tab-games"),
            Tab::Apps => t!("tab-apps"),
            Tab::Settings => t!("tab-settings"),
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Layer {
    Detail(String),
    Quick,
    Power,
    Keyboard,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Filter {
    #[default]
    All,
    Steam,
    Epic,
    Recent,
}

#[derive(Clone, Copy)]
pub struct Shell {
    pub tab: Signal<Tab>,
    pub layers: Signal<Vec<Layer>>,
    pub library: Signal<Library>,
    pub scanning: Signal<bool>,
    pub settings: Signal<Settings>,
    /// The title under focus, which drives the backdrop and accent colour.
    pub focused: Signal<Option<String>>,
    pub pad: Signal<Option<(String, Family)>>,
    pub toasts: Signal<Vec<(u64, String)>>,
    /// The app or game the user came from when they called JoyOS up.
    pub outside: Signal<Option<Outside>>,
    pub status: Signal<Status>,
    pub lang: Signal<usize>,
    pub search: Signal<String>,
    pub filter: Signal<Filter>,
    /// Bumped on every controller press; the setup screen's button tester listens.
    pub last_press: Signal<Option<Pad>>,
    /// 0.0 – 1.0, read when the quick menu opens.
    pub volume: Signal<Option<f32>>,
    /// `None` on screens that can't change brightness.
    pub brightness: Signal<Option<u8>>,
    bridge: document::Eval,
    toast_id: Signal<u64>,
}

impl Shell {
    fn send(&self, msg: Value) {
        let _ = self.bridge.send(msg);
    }

    pub fn sound(&self, kind: &str) {
        self.send(json!({ "t": "sound", "s": kind }));
    }

    pub fn push_prefs(&self) {
        let s = self.settings.read();
        let lang = i18n::current();
        self.send(json!({
            "t": "prefs",
            "sounds": s.sounds,
            "reduced": s.reduce_motion,
            "lang": lang.code,
            "rtl": lang.rtl,
        }));
    }

    // ----- navigation -----

    pub fn top(&self) -> Option<Layer> {
        self.layers.read().last().cloned()
    }

    pub fn open(mut self, layer: Layer) {
        if self.top().as_ref() == Some(&layer) {
            return;
        }
        if matches!(layer, Layer::Quick | Layer::Power) {
            self.sound("open");
        }
        // Power replaces the quick menu rather than stacking on it.
        if layer == Layer::Power && self.top() == Some(Layer::Quick) {
            self.layers.pop();
        }
        self.layers.push(layer);
    }

    pub fn close_layers(mut self) {
        self.layers.clear();
    }

    pub fn back(mut self) {
        if !self.settings.read().set_up {
            return;
        }
        if self.layers.pop().is_some() {
            self.sound("back");
        } else if *self.tab.read() != Tab::Home {
            self.sound("back");
            self.tab.set(Tab::Home);
        }
    }

    pub fn set_tab(mut self, tab: Tab) {
        if *self.tab.read() != tab {
            self.tab.set(tab);
        }
    }

    fn step_tab(self, delta: i32) {
        if !self.layers.read().is_empty() || !self.settings.read().set_up {
            return;
        }
        let i = (self.tab.read().index() as i32 + delta).rem_euclid(Tab::ALL.len() as i32);
        self.sound("tick");
        self.set_tab(Tab::ALL[i as usize]);
    }

    fn search(self) {
        if self.settings.read().set_up {
            self.set_tab(Tab::Games);
            self.open(Layer::Keyboard);
        }
    }

    fn more(self) {
        let id = self.focused.read().clone();
        if self.layers.read().is_empty()
            && let Some(id) = id
            && !id.starts_with("app:")
        {
            self.open(Layer::Detail(id));
        }
    }

    // ----- feedback -----

    pub fn toast(mut self, text: String) {
        let id = *self.toast_id.read() + 1;
        self.toast_id.set(id);
        self.toasts.push((id, text));
        let mut toasts = self.toasts;
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(3000)).await;
            toasts.retain(|(i, _)| *i != id);
        });
    }

    // ----- settings -----

    pub fn change(mut self, edit: impl FnOnce(&mut Settings)) {
        edit(&mut self.settings.write());
        let snapshot = self.settings.read().clone();
        apply_to_input(&snapshot);
        snapshot.save();
        self.push_prefs();
    }

    pub fn set_language(mut self, index: usize) {
        i18n::set_language(index);
        self.change(|s| s.language = Some(i18n::current().code.to_owned()));
        self.lang.set(index);
    }

    // ----- library -----

    pub fn find(&self, id: &str) -> Option<Title> {
        let lib = self.library.read();
        lib.games.iter().chain(&lib.apps).find(|t| t.id == id).cloned()
    }

    pub fn last_played(&self, title: &Title) -> Option<i64> {
        let ours = self.settings.read().played.get(&title.id).copied();
        title.last_played.max(ours)
    }

    /// Pinned first, then most recently played.
    pub fn jump_back_in(&self) -> Vec<Title> {
        let lib = self.library.read();
        let pinned = self.settings.read().pinned.clone();
        let mut out: Vec<Title> = pinned.iter().filter_map(|id| lib.games.iter().find(|g| &g.id == id).cloned()).collect();
        let mut rest: Vec<Title> = lib.games.iter().filter(|g| !pinned.contains(&g.id)).cloned().collect();
        rest.sort_by_key(|g| std::cmp::Reverse(self.last_played(g).unwrap_or(0)));
        out.extend(rest);
        out.truncate(16);
        out
    }

    pub fn rescan(mut self, announce: bool) {
        if *self.scanning.read() {
            return;
        }
        self.scanning.set(true);
        spawn(async move {
            if let Some(lib) = off_thread(Library::scan).await {
                let count = lib.games.len() as i64;
                self.library.set(lib);
                if announce {
                    self.toast(t!("toast-found", count = count));
                }
            }
            self.scanning.set(false);
        });
    }

    pub fn launch(self, title: &Title) {
        if !system::launch::open(&title.launch) {
            self.toast(t!("toast-failed", name = title.name.clone()));
            return;
        }
        // Apps aren't made for controllers, so the controller becomes a mouse there.
        SHARED.mouse_mode.store(title.source == Source::App, Ordering::Relaxed);
        crate::input::rumble();
        let id = title.id.clone();
        self.change(|s| {
            s.played.insert(id, now());
        });
        self.toast(t!("toast-starting", name = title.name.clone()));
        self.close_layers();
    }

    // ----- outside apps -----

    fn summon(mut self) {
        osk::send(osk::Msg::Hide);
        if let Some(mut front) = system::window::front_outside() {
            // Steam knows the real name of what's running; window titles are often junk.
            if let Some(game) = crate::library::running_steam_game().and_then(|id| self.find(&id)) {
                front.title = game.name;
            }
            self.outside.set(Some(front));
        }
        system::window::bring_us_forward();
        if self.top() != Some(Layer::Quick) {
            self.open(Layer::Quick);
        }
    }

    pub fn return_outside(self) {
        if let Some(o) = self.outside.read().clone() {
            self.close_layers();
            system::window::focus(&o);
        }
    }

    pub fn close_outside(mut self) {
        let target = self.outside.read().clone();
        if let Some(o) = target {
            system::window::close(&o);
            self.outside.set(None);
            SHARED.mouse_mode.store(false, Ordering::Relaxed);
        }
    }

    /// Shows the chosen on-screen keyboard over the app in front.
    pub fn show_keyboard(self, toggle: bool) {
        let s = self.settings.read().clone();
        match s.keyboard {
            Keyboard::Joy if osk::is_open() => {
                if toggle {
                    osk::send(osk::Msg::Hide);
                }
            }
            Keyboard::Joy => {
                let family = self.pad.read().as_ref().map(|(_, f)| *f).unwrap_or_default();
                osk::send(osk::Msg::Show { theme: s.theme, sounds: s.sounds, reduced: s.reduce_motion, family });
            }
            Keyboard::Windows => {
                spawn(async move {
                    off_thread(move || {
                        if toggle {
                            system::launch::touch_keyboard();
                        } else {
                            system::launch::show_touch_keyboard();
                        }
                    })
                    .await;
                });
            }
        }
    }

    pub fn screenshot(self) {
        let target = self.outside.read().clone();
        self.close_layers();
        spawn(async move {
            if let Some(o) = &target {
                system::window::focus(o);
            }
            tokio::time::sleep(Duration::from_millis(550)).await;
            system::inject::screenshot();
            if target.is_none() {
                self.toast(t!("toast-shot"));
            }
        });
    }

    // ----- input -----

    fn handle(mut self, event: InputEvent) {
        // While our keyboard is up over another app, the controller belongs to it.
        if osk::is_open() && !system::window::we_are_in_front() {
            match event {
                InputEvent::Move(d) => return osk::send(osk::Msg::Move(d)),
                InputEvent::Press(p) => return osk::send(osk::Msg::Press(p)),
                _ => {}
            }
        }
        match event {
            InputEvent::Move(d) => {
                let d = match d {
                    Dir::Up => "up",
                    Dir::Down => "down",
                    Dir::Left => "left",
                    Dir::Right => "right",
                };
                self.send(json!({ "t": "move", "d": d }));
            }
            InputEvent::Press(p) => {
                self.last_press.set(Some(p));
                match p {
                    Pad::A => self.send(json!({ "t": "activate" })),
                    Pad::B => self.back(),
                    // Same as the keyboard over other apps: X deletes, Y adds a space.
                    Pad::X if self.top() == Some(Layer::Keyboard) => ui::keyboard::backspace(self),
                    Pad::Y if self.top() == Some(Layer::Keyboard) => ui::keyboard::typed(self, " "),
                    Pad::X => self.more(),
                    Pad::Y => self.search(),
                    Pad::Lb => self.step_tab(-1),
                    Pad::Rb => self.step_tab(1),
                    Pad::Lt => self.send(json!({ "t": "page", "d": -1 })),
                    Pad::Rt => self.send(json!({ "t": "page", "d": 1 })),
                    Pad::Start => {
                        if self.top() == Some(Layer::Quick) {
                            self.back();
                        } else if self.settings.read().set_up {
                            self.open(Layer::Quick);
                        }
                    }
                    Pad::Select => {}
                }
            }
            InputEvent::Tilt(x, y) => self.send(json!({ "t": "tilt", "x": x, "y": y })),
            InputEvent::Summon => self.summon(),
            InputEvent::Keyboard { toggle } => self.show_keyboard(toggle),
            InputEvent::Connected(name, family) => {
                let known = self.pad.read().as_ref().map(|(n, _)| n.clone());
                if known.as_deref() != Some(&name) {
                    self.toast(t!("pad-connected", name = name.clone()));
                }
                self.pad.set(Some((name, family)));
            }
            InputEvent::Disconnected => self.pad.set(None),
        }
    }

    fn handle_js(self, msg: Value) {
        match msg["t"].as_str().unwrap_or_default() {
            "back" => self.back(),
            "tab" => self.step_tab(msg["d"].as_i64().unwrap_or(1) as i32),
            "quick" => self.open(Layer::Quick),
            "search" => self.search(),
            "more" => self.more(),
            "slide" => ui::sliders::slide(self, msg["id"].as_str().unwrap_or_default(), msg["d"].as_i64().unwrap_or(0) as i32),
            "type" => ui::keyboard::typed(self, msg["k"].as_str().unwrap_or_default()),
            "keyboard" => self.show_keyboard(true),
            "log" => tracing::warn!("webview: {}", msg["m"].as_str().unwrap_or_default()),
            _ => {}
        }
    }
}

#[component]
pub fn App() -> Element {
    let bridge = use_hook(|| document::eval(NAV_JS));
    let shell = use_context_provider(|| Shell {
        tab: Signal::new(Tab::Home),
        layers: Signal::new(vec![]),
        library: Signal::new(Library::default()),
        scanning: Signal::new(false),
        settings: Signal::new(Settings::load()),
        focused: Signal::new(None),
        pad: Signal::new(None),
        toasts: Signal::new(vec![]),
        outside: Signal::new(None),
        status: Signal::new(Status::default()),
        lang: Signal::new(i18n::current_index()),
        search: Signal::new(String::new()),
        filter: Signal::new(Filter::All),
        last_press: Signal::new(None),
        volume: Signal::new(None),
        brightness: Signal::new(None),
        bridge,
        toast_id: Signal::new(0),
    });

    use_hook(move || {
        use dioxus::desktop::tao::platform::windows::WindowExtWindows;
        system::window::set_ours(dioxus::desktop::window().window.hwnd());
        osk::create();
        shell.push_prefs();
        shell.rescan(false);
    });

    // The keyboard window would otherwise keep JoyOS running after the main one closes.
    let desktop = dioxus::desktop::use_window();
    let main_id = desktop.id();
    dioxus::desktop::use_wry_event_handler(move |event, _| {
        use dioxus::desktop::tao::event::{Event, WindowEvent};
        if let Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. } = event
            && *window_id == main_id
            && let Some(id) = osk::window_id()
        {
            desktop.close_window(id);
        }
    });

    use_future(move || async move {
        let rx = INPUT.lock().unwrap().take();
        if let Some(mut rx) = rx {
            while let Some(event) = rx.next().await {
                shell.handle(event);
            }
        }
    });

    use_future(move || async move {
        let mut bridge = bridge;
        while let Ok(msg) = bridge.recv::<Value>().await {
            shell.handle_js(msg);
        }
    });

    use_future(move || async move {
        let mut status = shell.status;
        loop {
            if let Some(s) = off_thread(system::status::read).await {
                status.set(s);
            }
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
    });

    let lang = *shell.lang.read();
    let set_up = shell.settings.read().set_up;
    let layers = shell.layers.read().clone();

    // The theme lives on <html> so CSS tokens cascade into everything, glass included.
    use_effect(move || {
        let s = shell.settings.read();
        let theme = if s.theme == Theme::Light { "light" } else { "dark" };
        let motion = if s.reduce_motion { "less" } else { "full" };
        document::eval(&format!(
            "document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.motion='{motion}';"
        ));
    });

    rsx! {
        ui::Backdrop {}
        // Keyed on language so every string re-renders after a switch.
        for l in [lang] {
            div { key: "{l}", class: "shell",
                ui::TopBar {}
                ui::View {}
                ui::HintBar {}
                for (i, layer) in layers.iter().enumerate() {
                    match layer {
                        Layer::Detail(id) => rsx! { ui::detail::Detail { key: "{i}-detail", id: id.clone() } },
                        Layer::Quick => rsx! { ui::quick::Quick { key: "{i}-quick" } },
                        Layer::Power => rsx! { ui::power::Power { key: "{i}-power" } },
                        Layer::Keyboard => rsx! { ui::keyboard::Keyboard { key: "{i}-keyboard" } },
                    }
                }
                if !set_up {
                    ui::setup::Setup {}
                }
            }
        }
        ui::Toasts {}
        div { class: "grain" }
    }
}
