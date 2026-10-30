//! Shell state and the root component. Controller events, JS messages and UI
//! actions all funnel through `Shell`, so behaviour is the same however you drive it.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt;
use serde_json::{Value, json};

use crate::household;
use crate::input::{Dir, Family, InputEvent, Pad, SHARED};
use crate::library::{Library, Source, Title};
use crate::license::{self, Plan};
use crate::settings::{GameProfile, Keyboard, Member, RomFolder, Scene, Session, Settings, Theme, WebTile};
use crate::system::{self, inject, off_thread, power, status::Status, window::Outside};
use crate::update::{self, Update};
use crate::{i18n, osk, plugins, sync, t, ui, words};

pub const NAV_JS: &str = include_str!("../assets/js/nav.js");

static INPUT: Mutex<Option<UnboundedReceiver<InputEvent>>> = Mutex::new(None);

/// The game or site being played, from launch until JoyOS is called back.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Running {
    /// Tells sessions apart, so a late power-plan answer reaches the right one.
    id: u64,
    game: String,
    start: i64,
    member: Option<String>,
    /// The power plan to put back, if the game changed it.
    restore: Option<String>,
    /// The "five minutes left" buzz has been given.
    #[serde(default)]
    warned: bool,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

/// A poisoned lock only means another thread panicked mid-update; the data is still usable.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The open session, kept on disk so a crash or restart doesn't lose the time played.
fn session_file() -> Option<std::path::PathBuf> {
    crate::settings::data_root().map(|d| d.join("session.json"))
}

fn save_running(run: Option<&Running>) {
    let Some(file) = session_file() else { return };
    match run.and_then(|r| serde_json::to_vec(r).ok()) {
        Some(bytes) => {
            let _ = crate::settings::write_atomic(&file, &bytes);
        }
        None => {
            let _ = std::fs::remove_file(file);
        }
    }
}

pub fn hand_over_input(rx: UnboundedReceiver<InputEvent>) {
    *lock(&INPUT) = Some(rx);
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
    Watch,
    Apps,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Home, Tab::Games, Tab::Watch, Tab::Apps, Tab::Settings];

    pub fn label(self) -> String {
        match self {
            Tab::Home => t!("tab-home"),
            Tab::Games => t!("tab-games"),
            Tab::Watch => t!("tab-watch"),
            Tab::Apps => t!("tab-apps"),
            Tab::Settings => t!("tab-settings"),
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

/// What a text entry is for. The keyboard fills `search`, and `finish_text` acts on it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TextFor {
    WebName,
    WebUrl(usize),
    Member,
    License,
    ArtKey,
    RomPath,
    RomEmulator(usize),
    /// What the emulator is given, with `{file}` for the game.
    RomArgs(usize),
    SyncFolder,
    UpdateUrl,
}

/// Why a PIN is being entered.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PinFor {
    /// Choosing this person as the one playing.
    Switch(String),
    /// Setting or clearing this person's PIN.
    Edit(String),
    /// Choosing "Everyone", which lifts all limits, so a grown-up's PIN is needed.
    Everyone,
    /// Opening the way back to Windows on a family PC.
    Recovery,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Layer {
    Detail(String),
    Quick,
    Power,
    Keyboard,
    Text(TextFor),
    Pin(PinFor),
    /// The way back to the Windows desktop, from the recovery buttons.
    Recovery,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Filter {
    #[default]
    All,
    Source(Source),
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
    /// Controller battery: level and whether it's charging. `None` when the pad doesn't say.
    pub battery: Signal<Option<(u8, bool)>>,
    /// A newer JoyOS, if the update manifest lists one.
    pub update: Signal<Option<Update>>,
    /// Bumped when a phone pairs or is forgotten, so the code on screen is redrawn.
    pub paired: Signal<u32>,
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

    /// Whether the keyboard is up, so the controller types rather than moves.
    pub fn typing(&self) -> bool {
        matches!(self.top(), Some(Layer::Keyboard) | Some(Layer::Text(_)))
    }

    pub fn open(mut self, layer: Layer) {
        if self.top().as_ref() == Some(&layer) {
            return;
        }
        // Setup sits on top of everything; layers opened under it would be stuck out of sight.
        if !self.settings.read().set_up {
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
        let was_text = matches!(self.top(), Some(Layer::Text(_)));
        if self.layers.pop().is_some() {
            if was_text {
                self.search.set(String::new());
            }
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
            && !id.starts_with("web:")
        {
            self.open(Layer::Detail(id));
        }
    }

    /// Opens a text entry, starting from `prefill`.
    pub fn ask(mut self, target: TextFor, prefill: &str) {
        self.search.set(prefill.to_owned());
        self.open(Layer::Text(target));
    }

    /// Closes a text entry and acts on what was typed.
    pub fn finish_text(mut self, target: TextFor) {
        let text = self.search.read().trim().to_owned();
        self.search.set(String::new());
        self.layers.pop();
        self.sound("select");
        match target {
            TextFor::WebName if !text.is_empty() => {
                let index = self.settings.read().web_tiles.len();
                self.change(|s| s.web_tiles.push(WebTile { name: text, url: String::new() }));
                self.ask(TextFor::WebUrl(index), "https://");
            }
            TextFor::WebUrl(i) => {
                let url = web_address(&text);
                let url = if url.is_empty() {
                    url
                } else if let Some(url) = system::launch::web_link(&url) {
                    url
                } else {
                    self.toast(t!("web-bad-address"));
                    String::new()
                };
                self.change(|s| {
                    if url.is_empty() {
                        if i < s.web_tiles.len() {
                            s.web_tiles.remove(i);
                        }
                    } else if let Some(tile) = s.web_tiles.get_mut(i) {
                        tile.url = url;
                    }
                });
            }
            TextFor::Member if !text.is_empty() => {
                let id = system::random::hex(8).map_or_else(|| format!("m{}", now()), |h| format!("m{h}"));
                self.change(|s| s.members.push(Member { id, name: text, ..Default::default() }));
                self.toast(t!("member-added"));
            }
            TextFor::License => match license::check(&text) {
                Some(l) => {
                    self.change(|s| s.license = text);
                    self.toast(t!("plan-activated", plan = plan_name(l.plan)));
                }
                None => self.toast(t!("plan-bad-key")),
            },
            TextFor::ArtKey => {
                self.change(|s| s.art_key = text);
                self.rescan(true);
            }
            TextFor::SyncFolder => {
                self.change(|s| s.sync_folder = (!text.is_empty()).then_some(text));
            }
            TextFor::UpdateUrl => {
                self.change(|s| s.update_url = text);
            }
            TextFor::RomPath if !text.is_empty() => {
                let index = self.settings.read().roms.len();
                self.change(|s| s.roms.push(RomFolder { path: text, ..Default::default() }));
                self.ask(TextFor::RomEmulator(index), "");
            }
            TextFor::RomEmulator(i) => {
                let asks_args = !text.is_empty();
                self.change(|s| {
                    if let Some(rom) = s.roms.get_mut(i) {
                        rom.emulator = text;
                    }
                });
                // Most emulators just take the file, so this starts filled in and can be left as is.
                if asks_args {
                    self.ask(TextFor::RomArgs(i), "{file}");
                } else {
                    self.rescan(true);
                }
            }
            TextFor::RomArgs(i) => {
                self.change(|s| {
                    if let Some(rom) = s.roms.get_mut(i) {
                        rom.args = if text == "{file}" { String::new() } else { text };
                    }
                });
                self.rescan(true);
            }
            _ => {}
        }
    }

    /// Whether choosing "Everyone" needs a grown-up's PIN: someone has a limit, and a grown-up has a PIN.
    pub fn everyone_needs_pin(&self) -> bool {
        let s = self.settings.read();
        s.members.iter().any(|m| m.daily_minutes.is_some()) && !grown_up_pins(&s.members).is_empty()
    }

    /// Sets, changes or clears a member's PIN, or switches to them. Returns whether it worked.
    pub fn enter_pin(mut self, target: PinFor, pin: &str) -> bool {
        let wait = household::pin_wait(now());
        if wait > 0 {
            self.toast(t!("pin-wait", seconds = wait));
            return false;
        }
        let settings = self.settings.read().clone();
        // Who the PIN has to match, for the checks below.
        let candidates: Vec<&Member> = match &target {
            PinFor::Switch(id) => household::member(&settings.members, id).into_iter().filter(|m| m.pin_hash.is_some()).collect(),
            PinFor::Everyone | PinFor::Recovery => grown_up_pins(&settings.members),
            PinFor::Edit(_) => vec![],
        };
        let matched = candidates.iter().find(|m| m.pin_hash.as_deref().is_some_and(|h| household::check_pin(h, pin)));
        if !candidates.is_empty() {
            household::pin_tried(matched.is_some(), now());
            let Some(m) = matched else {
                self.toast(t!("pin-wrong"));
                return false;
            };
            // Older files used a quick hash; swap in the slow one now that we know the PIN.
            if m.pin_hash.as_deref().is_some_and(household::is_old_hash)
                && let Some(fresh) = household::new_pin_hash(pin)
            {
                let id = m.id.clone();
                self.change(|s| {
                    if let Some(m) = s.members.iter_mut().find(|m| m.id == id) {
                        m.pin_hash = Some(fresh);
                    }
                });
            }
        }
        self.layers.pop();
        match target {
            PinFor::Switch(id) => {
                let name = household::member(&settings.members, &id).map(|m| m.name.clone()).unwrap_or_default();
                self.change(|s| s.playing = Some(id));
                self.toast(t!("pin-switched", name = name));
            }
            PinFor::Everyone => {
                self.change(|s| s.playing = None);
                self.toast(t!("pin-switched", name = t!("family-everyone")));
            }
            PinFor::Recovery => self.layers.push(Layer::Recovery),
            PinFor::Edit(id) => {
                let hash = if pin.is_empty() {
                    None
                } else if let Some(hash) = household::new_pin_hash(pin) {
                    Some(hash)
                } else {
                    self.toast(t!("pin-not-saved"));
                    return false;
                };
                self.change(|s| {
                    if let Some(m) = s.members.iter_mut().find(|m| m.id == id) {
                        m.pin_hash = hash;
                    }
                });
                self.toast(t!("pin-saved"));
            }
        }
        self.sound("select");
        true
    }

    // ----- feedback -----

    pub fn toast(mut self, text: String) {
        let id = *self.toast_id.read() + 1;
        self.toast_id.set(id);
        self.toasts.push((id, text));
        let mut toasts = self.toasts;
        // Tied to the root, not to whatever screen asked: that screen may close first.
        spawn_forever(async move {
            tokio::time::sleep(Duration::from_millis(3000)).await;
            toasts.retain(|(i, _)| *i != id);
        });
    }

    // ----- settings -----

    pub fn change(mut self, edit: impl FnOnce(&mut Settings)) {
        edit(&mut self.settings.write());
        let snapshot = self.settings.read().clone();
        apply_to_input(&snapshot);
        crate::settings::save_soon(snapshot);
        self.push_prefs();
    }

    /// Replaces every setting, e.g. after a sync pulled a newer copy.
    fn replace_settings(mut self, fresh: Settings) {
        apply_to_input(&fresh);
        self.settings.set(fresh);
        self.push_prefs();
    }

    pub fn set_language(mut self, index: usize) {
        i18n::set_language(index);
        self.change(|s| s.language = Some(i18n::current().code.to_owned()));
        self.lang.set(index);
    }

    /// Listens once and adds what was said to the text being typed.
    pub fn voice(self) {
        self.toast(t!("voice-listening"));
        let language = i18n::current().code;
        spawn_forever(async move {
            match off_thread(move || system::voice::listen(language)).await.flatten() {
                Some(text) => {
                    let mut search = self.search;
                    let mut typed = search.write();
                    if !typed.is_empty() && !typed.ends_with(' ') {
                        typed.push(' ');
                    }
                    typed.push_str(text.trim());
                }
                None => self.toast(t!("voice-unavailable")),
            }
        });
    }

    /// Remembers the words typed in `text`, for suggestions next time.
    pub fn remember_words(self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let text = text.to_owned();
        self.change(|s| words::remember(s, &text));
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

    /// Pinned first, then the games that have been played, most recent first.
    pub fn jump_back_in(&self) -> Vec<Title> {
        let lib = self.library.read();
        let pinned = self.settings.read().pinned.clone();
        let mut out: Vec<Title> = pinned.iter().filter_map(|id| lib.games.iter().find(|g| &g.id == id).cloned()).collect();
        let mut rest: Vec<(i64, Title)> = lib
            .games
            .iter()
            .filter(|g| !pinned.contains(&g.id))
            .filter_map(|g| self.last_played(g).map(|t| (t, g.clone())))
            .collect();
        rest.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
        out.extend(rest.into_iter().map(|(_, g)| g));
        out.truncate(16);
        out
    }

    /// Finds games and apps, shows them, then fills in missing covers online.
    pub fn rescan(mut self, announce: bool) {
        if *self.scanning.read() {
            return;
        }
        self.scanning.set(true);
        let (art_key, roms) = {
            let s = self.settings.read();
            (s.art_key.clone(), s.roms.clone())
        };
        spawn_forever(async move {
            if let Some(lib) = off_thread(move || Library::scan(&roms)).await {
                let count = lib.games.len() as i64;
                let wants_art = lib.missing_art() && !art_key.trim().is_empty();
                self.library.set(lib.clone());
                if announce {
                    self.toast(t!("toast-found", count = count));
                }
                if wants_art && let Some((lib, error)) = off_thread(move || lib.with_art(&art_key)).await {
                    self.library.set(lib);
                    if error == Some(crate::library::ArtError::BadKey) {
                        self.toast(t!("art-bad-key"));
                    }
                }
            }
            self.scanning.set(false);
        });
    }

    /// Minutes `member` may still play today, counting the game that's running. `None` without a limit.
    fn minutes_left(&self, member: &Member) -> Option<i64> {
        let limit = member.daily_minutes? as i64;
        let since = now() - household::seconds_into_local_day();
        let used = household::minutes_since(&self.settings.read().sessions, Some(&member.id), since) as i64;
        let running = lock(&RUNNING)
            .as_ref()
            .filter(|r| r.member.as_deref() == Some(&member.id))
            .map_or(0, |r| (now() - r.start.max(since)).max(0) / 60);
        Some(limit - used - running)
    }

    /// Checked every minute while someone with a limit plays: a buzz near the end, then JoyOS comes back.
    fn check_limit(self) {
        let (member, warned) = {
            let run = lock(&RUNNING);
            let Some(run) = run.as_ref() else { return };
            let s = self.settings.read();
            (run.member.as_deref().and_then(|id| household::member(&s.members, id)).cloned(), run.warned)
        };
        let Some(member) = member else { return };
        let Some(left) = self.minutes_left(&member) else { return };
        if left <= 0 {
            self.summon();
            self.toast(t!("toast-time-up", name = member.name));
        } else if left <= 5 && !warned {
            if let Some(run) = lock(&RUNNING).as_mut() {
                run.warned = true;
            }
            crate::input::rumble();
        }
        // The file's time marks how long the session lasted, should JoyOS stop without saying.
        save_running(lock(&RUNNING).as_ref());
    }

    pub fn launch(self, title: &Title) {
        let settings = self.settings.read().clone();
        let member = settings.playing.as_deref().and_then(|id| household::member(&settings.members, id)).cloned();
        if let Some(m) = &member
            && self.minutes_left(m).is_some_and(|left| left <= 0)
        {
            self.toast(t!("toast-time-up", name = m.name.clone()));
            return;
        }

        // Per-game settings are a Plus feature; on the free plan they're ignored.
        let profile = if license::has_plus(&settings.license) {
            settings.profiles.get(&title.id).copied().unwrap_or_default()
        } else {
            GameProfile::default()
        };
        let folder = title.folder.as_deref();
        let opened = match title.emulator.as_deref() {
            Some(exe) => {
                let args = system::launch::emulator_args(title.args.as_deref().unwrap_or_default(), &title.launch);
                system::launch::open_in(exe, Some(&args), Some(Path::new(&title.launch)).and_then(Path::parent))
            }
            None => system::launch::open_in(&title.launch, title.args.as_deref(), folder),
        };
        if !opened {
            self.toast(t!("toast-failed", name = title.name.clone()));
            return;
        }
        // Apps aren't made for controllers, so the controller becomes a mouse there.
        SHARED.mouse_mode.store(title.source == Source::App, Ordering::Relaxed);
        SHARED.swap_ab.store(profile.swap_ab.unwrap_or(settings.swap_ab), Ordering::Relaxed);
        crate::input::rumble();
        self.begin_session(title.id.clone(), member.map(|m| m.id), profile.power);
        let id = title.id.clone();
        self.change(|s| {
            s.played.insert(id, now());
        });
        self.toast(t!("toast-starting", name = title.name.clone()));
        self.close_layers();
    }

    /// Opens a website full screen, in Edge, the way the media hub shows it.
    pub fn launch_web(self, tile: &WebTile) {
        if !system::launch::open_kiosk(&tile.url) {
            self.toast(t!("toast-failed", name = tile.name.clone()));
            return;
        }
        SHARED.mouse_mode.store(true, Ordering::Relaxed);
        let member = self.settings.read().playing.clone();
        self.begin_session(format!("web:{}", tile.url), member, None);
        self.close_layers();
    }

    /// Starts counting play time, and applies the game's power plan if it has one.
    fn begin_session(self, game: String, member: Option<String>, power_mode: Option<crate::settings::PowerMode>) {
        // Launching while something is still counted closes that one first.
        self.end_session();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let run = Running { id, game, start: now(), member, restore: None, warned: false };
        save_running(Some(&run));
        *lock(&RUNNING) = Some(run);
        if let Some(mode) = power_mode {
            spawn_forever(async move {
                let before = off_thread(move || power::set_mode(mode)).await.flatten();
                let mut running = lock(&RUNNING);
                match running.as_mut() {
                    Some(run) if run.id == id => {
                        run.restore = before;
                        save_running(Some(run));
                    }
                    // The session already ended: put the plan straight back.
                    _ => {
                        drop(running);
                        if let Some(plan) = before {
                            std::thread::spawn(move || power::restore_plan(&plan));
                        }
                    }
                }
            });
        }
    }

    /// Ends the session when JoyOS is called back: records the time and undoes the game's settings.
    fn end_session(self) {
        let Some(run) = lock(&RUNNING).take() else { return };
        save_running(None);
        self.record_run(run, now());
    }

    fn record_run(self, run: Running, end: i64) {
        let seconds = (end - run.start).clamp(0, u32::MAX as i64) as u32;
        let offset = household::local_offset(now());
        self.change(|s| {
            let session = Session { member: run.member, game: run.game, start: run.start, seconds, minutes: 0 };
            household::record(&mut s.sessions, session, offset);
        });
        if let Some(plan) = run.restore {
            spawn_forever(async move {
                off_thread(move || power::restore_plan(&plan)).await;
            });
        }
    }

    /// A session left open by a crash or restart is counted up to when it was last seen.
    fn recover_session(self) {
        let Some(file) = session_file() else { return };
        let Ok(bytes) = std::fs::read(&file) else { return };
        let seen = std::fs::metadata(&file).and_then(|m| m.modified()).ok();
        let _ = std::fs::remove_file(&file);
        if let Ok(run) = serde_json::from_slice::<Running>(&bytes) {
            let end = seen.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(run.start, |d| d.as_secs() as i64);
            self.record_run(run, end.min(now()));
        }
    }

    pub fn sync_now(self) {
        let Some(folder) = self.settings.read().sync_folder.clone() else { return };
        let Some(local) = crate::settings::file_path() else { return };
        let Some(state) = crate::settings::data_root().map(|d| d.join("sync-state")) else { return };
        spawn_forever(async move {
            let result = off_thread(move || {
                // Unsaved changes go out first, so a pull can't drop them.
                crate::settings::flush();
                sync::sync(Path::new(&folder), &local, &state)
            })
            .await;
            match result {
                Some(Ok(sync::Outcome::Pulled)) => {
                    self.replace_settings(Settings::load());
                    self.toast(t!("sync-pulled"));
                }
                Some(Ok(sync::Outcome::Pushed)) => self.toast(t!("sync-pushed")),
                Some(Ok(sync::Outcome::UpToDate)) => self.toast(t!("sync-same")),
                _ => self.toast(t!("sync-failed")),
            }
        });
    }

    // ----- outside apps -----

    fn summon(mut self) {
        osk::send(osk::Msg::Hide);
        self.end_session();
        crate::settings::flush();
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
                osk::send(osk::Msg::Show {
                    look: look(&s),
                    sounds: s.sounds,
                    reduced: s.reduce_motion,
                    family,
                    words: s.words.clone(),
                });
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

    /// Records the last 30 seconds of play with the Windows Game Bar.
    pub fn save_clip(self) {
        spawn(async move {
            off_thread(inject::game_clip).await;
        });
        self.toast(t!("toast-clip"));
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
                    Pad::X if self.typing() => ui::keyboard::backspace(self),
                    Pad::Y if self.typing() => ui::keyboard::typed(self, " "),
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
            InputEvent::Recovery => self.recovery(),
            InputEvent::Paired => {
                let mut paired = self.paired;
                paired += 1;
                self.toast(t!("phone-paired"));
            }
            InputEvent::Keyboard { toggle } => self.show_keyboard(toggle),
            InputEvent::Connected(name, family) => {
                let known = self.pad.read().as_ref().map(|(n, _)| n.clone());
                if known.as_deref() != Some(&name) {
                    self.toast(t!("pad-connected", name = name.clone()));
                }
                self.pad.set(Some((name, family)));
            }
            InputEvent::Disconnected => self.pad.set(None),
            InputEvent::Text(text) => {
                if system::window::we_are_in_front() {
                    ui::keyboard::typed(self, &text);
                } else {
                    inject::text(&text);
                }
            }
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
            "recovery" => self.recovery(),
            // Before setup, follow the system's "reduce motion"; after that, the user's own choice.
            "os-motion" if !self.settings.read().set_up => {
                let reduced = msg["reduced"].as_bool().unwrap_or(false);
                if self.settings.read().reduce_motion != reduced {
                    self.change(|s| s.reduce_motion = reduced);
                }
            }
            "plugin-toast" => self.toast(msg["m"].as_str().unwrap_or_default().chars().take(160).collect()),
            "plugin-open" => {
                let url = msg["u"].as_str().unwrap_or_default();
                if plugins::allowed_link(url) {
                    system::launch::open(url);
                }
            }
            "log" => tracing::warn!("webview: {}", msg["m"].as_str().unwrap_or_default()),
            _ => {}
        }
    }

    /// Brings JoyOS forward with the way back to Windows. On a family PC, a grown-up's PIN comes first.
    pub fn recovery(mut self) {
        if !self.settings.read().set_up {
            return;
        }
        osk::send(osk::Msg::Hide);
        system::window::bring_us_forward();
        let layer = if grown_up_pins(&self.settings.read().members).is_empty() {
            Layer::Recovery
        } else {
            Layer::Pin(PinFor::Recovery)
        };
        self.layers.clear();
        self.open(layer);
    }

    /// Checks for a newer JoyOS once a start, if the user has set an update address.
    fn check_update(mut self) {
        let (url, test_versions) = {
            let s = self.settings.read();
            (s.update_url.clone(), s.test_versions)
        };
        if url.trim().is_empty() {
            return;
        }
        spawn(async move {
            if let Some(Some(update)) = off_thread(move || update::check(&url, test_versions)).await {
                self.toast(t!("update-available", version = update.version.clone()));
                self.update.set(Some(update));
            }
        });
    }
}

/// Members who have a PIN and no limit: the grown-ups, whose PIN unlocks "Everyone" and recovery.
fn grown_up_pins(members: &[Member]) -> Vec<&Member> {
    members.iter().filter(|m| m.pin_hash.is_some() && m.daily_minutes.is_none()).collect()
}

/// A web address as typed: `netflix.com` becomes `https://netflix.com`.
pub fn web_address(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        String::new()
    } else if text.contains("://") {
        text.to_owned()
    } else {
        format!("https://{text}")
    }
}

pub fn plan_name(plan: Plan) -> String {
    match plan {
        Plan::Free => t!("plan-free"),
        Plan::Plus => t!("plan-plus"),
        Plan::Business => t!("plan-business"),
    }
}

/// The attributes the stylesheet reads, derived from settings. Auto follows the clock.
pub fn look(s: &Settings) -> String {
    let dark = match s.theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::Auto => {
            let hour = ui::hour();
            !(7..19).contains(&hour)
        }
    };
    let theme = if dark { "dark" } else { "light" };
    let motion = if s.reduce_motion { "less" } else { "full" };
    let contrast = if s.high_contrast { "high" } else { "normal" };
    let text = if s.large_text { "large" } else { "normal" };
    let scene = match s.scene {
        Scene::Still => "still",
        Scene::Aurora => "aurora",
    };
    format!(
        "document.documentElement.dataset.theme='{theme}';document.documentElement.dataset.motion='{motion}';document.documentElement.dataset.contrast='{contrast}';document.documentElement.dataset.text='{text}';document.documentElement.dataset.scene='{scene}';"
    )
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
        battery: Signal::new(None),
        update: Signal::new(None),
        paired: Signal::new(0),
        bridge,
        toast_id: Signal::new(0),
    });

    use_hook(move || {
        use dioxus::desktop::tao::platform::windows::WindowExtWindows;
        system::window::set_ours(dioxus::desktop::window().window.hwnd());
        osk::create();
        shell.push_prefs();
        shell.recover_session();
        shell.rescan(false);
        shell.check_update();
        shell.start_sync_on_launch();
        shell.load_plugins();
    });

    // The keyboard window would otherwise keep JoyOS running after the main one closes.
    let desktop = dioxus::desktop::use_window();
    let main_id = desktop.id();
    dioxus::desktop::use_wry_event_handler(move |event, _| {
        use dioxus::desktop::tao::event::{Event, WindowEvent};
        if let Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. } = event
            && *window_id == main_id
        {
            shell.end_session();
            crate::settings::flush();
            if let Some(id) = osk::window_id() {
                desktop.close_window(id);
            }
        }
    });

    // Daily limits are watched during play, not only at launch.
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            shell.check_limit();
        }
    });

    use_future(move || async move {
        let rx = lock(&INPUT).take();
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
        let mut battery = shell.battery;
        loop {
            if let Some(s) = off_thread(system::status::read).await {
                status.set(s);
            }
            let level = SHARED.battery.load(Ordering::Relaxed);
            let charging = SHARED.charging.load(Ordering::Relaxed);
            battery.set((level != 255).then_some((level, charging)));
            tokio::time::sleep(Duration::from_secs(20)).await;
        }
    });

    let lang = *shell.lang.read();
    let set_up = shell.settings.read().set_up;
    let layers = shell.layers.read().clone();

    // The look lives on <html> so CSS tokens cascade into everything, glass included.
    // It reads the status signal too, so Auto theme re-checks the clock every 20 seconds.
    use_effect(move || {
        let _ = shell.status.read();
        let attrs = look(&shell.settings.read());
        document::eval(&attrs);
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
                        Layer::Keyboard => rsx! { ui::keyboard::Keyboard { key: "{i}-keyboard", target: None } },
                        Layer::Text(target) => rsx! { ui::keyboard::Keyboard { key: "{i}-text", target: Some(target.clone()) } },
                        Layer::Pin(target) => rsx! { ui::pin::Pin { key: "{i}-pin", target: target.clone() } },
                        Layer::Recovery => rsx! { ui::recovery::Recovery { key: "{i}-recovery" } },
                    }
                }
            }
        }
        // Outside the language-keyed shell, so picking a language doesn't restart setup.
        if !set_up {
            ui::setup::Setup {}
        }
        ui::Toasts {}
        div { class: "grain" }
    }
}

impl Shell {
    /// Pulls settings from the sync folder when it has a newer copy, once per start.
    fn start_sync_on_launch(self) {
        let (folder, plus) = {
            let s = self.settings.read();
            (s.sync_folder.clone(), license::has_plus(&s.license))
        };
        if folder.is_some() && plus {
            self.sync_now();
        }
    }

    /// Runs the plugins in `plugins\` when plugins are on and the plan covers them.
    fn load_plugins(self) {
        let (enabled, license_key) = {
            let s = self.settings.read();
            (s.plugins, s.license.clone())
        };
        if !enabled || !license::has_plus(&license_key) {
            return;
        }
        spawn(async move {
            let Some(found) = off_thread(plugins::load).await else { return };
            for plugin in found {
                let mut eval = document::eval(&plugins::wrap(&plugin.name, &plugin.script));
                spawn_forever(async move {
                    while let Ok(msg) = eval.recv::<Value>().await {
                        // A sandboxed plugin can only ask for these two things.
                        match msg["t"].as_str() {
                            Some("plugin-toast" | "plugin-open") => self.handle_js(msg),
                            Some("log") => tracing::warn!("plugin {}: {}", plugin.name, msg["m"].as_str().unwrap_or_default()),
                            _ => {}
                        }
                    }
                });
            }
        });
    }
}
