//! A phone remote for the couch. The phone joins the same network, opens the
//! address shown in Settings, and enters the code once. After that one WebSocket
//! carries its button presses and typing, and tells the phone when a text box is up.
//! It's off by default; turning it off closes the port and forgets every phone.
//!
//! What keeps it to this home: only IP addresses are accepted as the host name (so a
//! web page can't reach it by pointing its own name at this PC), the socket and the
//! code form only take requests from the remote's own page, wrong codes lock the pad
//! for a minute, and the code changes after each phone pairs.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use futures_channel::mpsc::UnboundedSender;
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_UP};

use crate::input::{Dir, InputEvent, Pad, SHARED};
use crate::system::{inject, random, window};
use crate::t;

pub const PORT: u16 = 47621;

const PAIR_PAGE: &str = include_str!("../assets/remote/pair.html");
const REMOTE_PAGE: &str = include_str!("../assets/remote/remote.html");

/// Wrong codes allowed before pairing waits, and for how long.
const TRIES: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(60);

struct State {
    pin: String,
    /// Cookies of the phones paired since the remote was turned on.
    tokens: HashSet<String>,
    misses: u32,
    locked_until: Option<Instant>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    input: UnboundedSender<InputEvent>,
    /// Which start this is, so a server that's still closing can't clear a newer one.
    run: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn state() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The code to show on the TV, while the remote is on.
pub fn pin() -> Option<String> {
    state().as_ref().map(|s| s.pin.clone())
}

/// Starts listening on the local network. Does nothing if it's already running.
pub fn start(input: UnboundedSender<InputEvent>) {
    let mut guard = state();
    if guard.is_some() {
        return;
    }
    let Some(pin) = random::digits(6) else {
        tracing::warn!("phone remote stays off: no random numbers for a code");
        return;
    };
    static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let run = RUNS.fetch_add(1, Ordering::Relaxed);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    *guard = Some(State { pin, tokens: HashSet::new(), misses: 0, locked_until: None, stop: Some(stop), input, run });
    drop(guard);
    let ended = move || {
        let mut guard = state();
        if guard.as_ref().is_some_and(|s| s.run == run) {
            *guard = None;
        }
    };
    let spawned = std::thread::Builder::new().name("companion".into()).spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
        match runtime {
            Ok(rt) => rt.block_on(serve(stopped)),
            Err(e) => tracing::warn!("phone remote: {e}"),
        }
        // However it ended, the remote is off now.
        ended();
    });
    if spawned.is_err() {
        tracing::warn!("couldn't start the phone remote");
        *state() = None;
    }
}

/// Closes the port and forgets every paired phone.
pub fn stop() {
    if let Some(stop) = state().take().and_then(|mut s| s.stop.take()) {
        let _ = stop.send(());
    }
}

/// Phones paired so far have to pair again with a fresh code.
pub fn forget_phones() {
    if let Some(s) = state().as_mut() {
        s.tokens.clear();
        if let Some(pin) = random::digits(6) {
            s.pin = pin;
        }
    }
}

/// This PC's address on the local network, so the phone knows where to go.
pub fn lan_address() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // Connecting a UDP socket sends nothing; it only makes the OS pick the outgoing route.
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

async fn serve(stopped: tokio::sync::oneshot::Receiver<()>) {
    // Turning it off and on quickly can find the port still closing; give it a moment.
    let mut listener = None;
    for _ in 0..10 {
        match tokio::net::TcpListener::bind(("0.0.0.0", PORT)).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => {
                tracing::debug!("phone remote waiting for port {PORT}: {e}");
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
    let Some(listener) = listener else {
        tracing::warn!("phone remote can't listen on port {PORT}");
        return;
    };
    let app = Router::new()
        .route("/", get(page))
        .route("/pair", post(pair))
        .route("/ws", get(socket))
        .layer(DefaultBodyLimit::max(4096));
    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = stopped.await;
        })
        .await;
}

/// The host has to be a plain IP address with our port. A web page that points its own
/// name at this PC (DNS rebinding) sends that name instead, and is turned away.
fn host_ok(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|h| h.to_str().ok()) else { return false };
    let Some((ip, port)) = host.rsplit_once(':') else { return false };
    port == PORT.to_string() && ip.trim_matches(['[', ']']).parse::<IpAddr>().is_ok()
}

/// Requests that change something must come from the remote's own page.
fn origin_ok(headers: &HeaderMap) -> bool {
    let host = headers.get(header::HOST).and_then(|h| h.to_str().ok());
    let origin = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok());
    matches!((host, origin), (Some(host), Some(origin)) if origin == format!("http://{host}"))
}

fn paired(headers: &HeaderMap) -> bool {
    let cookies = headers.get_all(header::COOKIE).iter().filter_map(|c| c.to_str().ok()).flat_map(|c| c.split(';'));
    let guard = state();
    let Some(s) = guard.as_ref() else { return false };
    cookies.filter_map(|c| c.trim().strip_prefix("joy=")).any(|t| s.tokens.iter().any(|k| bool::from(k.as_bytes().ct_eq(t.as_bytes()))))
}

fn refuse(status: StatusCode) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], "").into_response()
}

async fn page(headers: HeaderMap) -> Response {
    if !host_ok(&headers) {
        return refuse(StatusCode::FORBIDDEN);
    }
    let body = if paired(&headers) { REMOTE_PAGE } else { PAIR_PAGE };
    let html = body.replace("/*WORDS*/{}", &words().to_string()).replace("{lang}", crate::i18n::current().code);
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; form-action 'self'; frame-ancestors 'none'"),
        ],
        Html(html),
    )
        .into_response()
}

/// The remote's words, in the TV's language.
fn words() -> Value {
    json!({
        "pairTitle": t!("remote-pair-title"),
        "pairBody": t!("remote-pair-body"),
        "pairButton": t!("remote-pair-button"),
        "wrong": t!("remote-wrong"),
        "wait": t!("remote-wait"),
        "title": t!("remote-title"),
        "move": t!("remote-move"),
        "press": t!("remote-press"),
        "type": t!("remote-type"),
        "send": t!("remote-send"),
        "lost": t!("remote-lost"),
        "typing": t!("remote-typing"),
        "lb": t!("remote-lb"),
        "rb": t!("remote-rb"),
        "view": t!("remote-view"),
        "menu": t!("remote-menu"),
        "home": t!("remote-home"),
    })
}

async fn pair(headers: HeaderMap, body: String) -> Response {
    if !host_ok(&headers) || !origin_ok(&headers) {
        return refuse(StatusCode::FORBIDDEN);
    }
    let entered = form(&body, "pin");
    let mut guard = state();
    let Some(s) = guard.as_mut() else { return refuse(StatusCode::SERVICE_UNAVAILABLE) };
    match check_code(s, &entered, Instant::now()) {
        Code::Right => {
            let Some(token) = random::hex(16) else { return refuse(StatusCode::SERVICE_UNAVAILABLE) };
            s.tokens.insert(token.clone());
            // A new code for the next phone, so one seen over a shoulder is no use later.
            if let Some(pin) = random::digits(6) {
                s.pin = pin;
            }
            let _ = s.input.unbounded_send(InputEvent::Paired);
            let cookie = format!("joy={token}; HttpOnly; SameSite=Strict; Path=/");
            ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
        }
        Code::Wrong => Redirect::to("/?wrong").into_response(),
        Code::Wait => Redirect::to("/?wait").into_response(),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Code {
    Right,
    Wrong,
    Wait,
}

fn check_code(s: &mut State, entered: &str, now: Instant) -> Code {
    if s.locked_until.is_some_and(|until| now < until) {
        return Code::Wait;
    }
    if bool::from(s.pin.as_bytes().ct_eq(entered.as_bytes())) {
        s.misses = 0;
        s.locked_until = None;
        return Code::Right;
    }
    s.misses += 1;
    if s.misses >= TRIES {
        s.misses = 0;
        s.locked_until = Some(now + LOCKOUT);
    }
    Code::Wrong
}

async fn socket(headers: HeaderMap, upgrade: WebSocketUpgrade) -> Response {
    if !host_ok(&headers) || !origin_ok(&headers) {
        return refuse(StatusCode::FORBIDDEN);
    }
    if !paired(&headers) {
        return refuse(StatusCode::UNAUTHORIZED);
    }
    upgrade.max_message_size(4096).on_upgrade(talk)
}

/// One phone's connection: its presses come in, and "a text box is up" goes out.
async fn talk(mut ws: WebSocket) {
    let mut typing = None;
    let mut tick = tokio::time::interval(Duration::from_millis(400));
    loop {
        tokio::select! {
            msg = ws.recv() => {
                let Some(Ok(msg)) = msg else { return };
                if let Message::Text(text) = msg
                    && let Ok(value) = serde_json::from_str::<Value>(text.as_str())
                {
                    act(&value);
                }
            }
            _ = tick.tick() => {
                if state().is_none() {
                    return;
                }
                let now = SHARED.typing.load(Ordering::Relaxed);
                if typing != Some(now) {
                    typing = Some(now);
                    if ws.send(Message::Text(json!({ "typing": now }).to_string().into())).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

fn act(msg: &Value) {
    let Some(input) = state().as_ref().map(|s| s.input.clone()) else { return };
    if let Some(name) = msg["k"].as_str() {
        if let Some(event) = route_key(name, window::we_are_in_front(), SHARED.typing.load(Ordering::Relaxed)) {
            let _ = input.unbounded_send(event);
        } else if let Some(vk) = outside_key(name) {
            inject::key(vk, true);
            inject::key(vk, false);
        }
    } else if let Some(text) = msg["t"].as_str().filter(|t| !t.is_empty()) {
        let _ = input.unbounded_send(InputEvent::Text(text.chars().take(500).collect()));
    } else if let Some([dx, dy]) = msg["m"].as_array().map(|a| a.as_slice()) {
        // The trackpad moves the pointer only over other apps; JoyOS itself is driven by focus.
        if !window::we_are_in_front() {
            let clamp = |v: &Value| v.as_f64().unwrap_or(0.0).clamp(-200.0, 200.0) as i32;
            inject::move_by(clamp(dx), clamp(dy));
        }
    } else if msg["c"].as_str() == Some("tap") && !window::we_are_in_front() {
        inject::button(inject::MouseButton::Left, true);
        inject::button(inject::MouseButton::Left, false);
    }
}

/// What a phone button does. While JoyOS is in front (or its keyboard is up), the same
/// as the controller. Over another app, only Home and the keyboard reach JoyOS;
/// the rest become keys for that app (see `outside_key`).
fn route_key(name: &str, in_front: bool, typing: bool) -> Option<InputEvent> {
    if name == "home" {
        return Some(InputEvent::Summon);
    }
    if !in_front && !typing {
        return match name {
            "start" => Some(InputEvent::Summon),
            "y" => Some(InputEvent::Keyboard { toggle: true }),
            _ => None,
        };
    }
    Some(match name {
        "up" => InputEvent::Move(Dir::Up),
        "down" => InputEvent::Move(Dir::Down),
        "left" => InputEvent::Move(Dir::Left),
        "right" => InputEvent::Move(Dir::Right),
        "a" => InputEvent::Press(Pad::A),
        "b" => InputEvent::Press(Pad::B),
        "x" => InputEvent::Press(Pad::X),
        "y" => InputEvent::Press(Pad::Y),
        "lb" => InputEvent::Press(Pad::Lb),
        "rb" => InputEvent::Press(Pad::Rb),
        "start" => InputEvent::Press(Pad::Start),
        "select" => InputEvent::Press(Pad::Select),
        _ => return None,
    })
}

/// Phone buttons over another app: arrows move, A is Enter, B and View are Esc.
fn outside_key(name: &str) -> Option<VIRTUAL_KEY> {
    Some(match name {
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "a" => VK_RETURN,
        "b" | "select" => VK_ESCAPE,
        _ => return None,
    })
}

/// The value of `key` in an `application/x-www-form-urlencoded` body.
fn form(body: &str, key: &str) -> String {
    body.split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| decode(v))
        .unwrap_or_default()
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                match std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(header::HeaderName, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(k.clone(), v.parse().unwrap());
        }
        h
    }

    #[test]
    fn forms_decode() {
        assert_eq!(form("pin=123456", "pin"), "123456");
        assert_eq!(form("t=hello+world%21&k=a", "t"), "hello world!");
        assert_eq!(form("t=100%", "t"), "100%");
        assert_eq!(form("k=a", "missing"), "");
    }

    #[test]
    fn only_ip_hosts_and_our_own_page_get_in() {
        assert!(host_ok(&headers(&[(header::HOST, "192.168.1.20:47621")])));
        assert!(!host_ok(&headers(&[(header::HOST, "evil.example:47621")])), "a rebound name is refused");
        assert!(!host_ok(&headers(&[(header::HOST, "192.168.1.20:80")])));
        assert!(!host_ok(&HeaderMap::new()));
        let same = headers(&[(header::HOST, "192.168.1.20:47621"), (header::ORIGIN, "http://192.168.1.20:47621")]);
        assert!(origin_ok(&same));
        let other = headers(&[(header::HOST, "192.168.1.20:47621"), (header::ORIGIN, "http://evil.example")]);
        assert!(!origin_ok(&other));
        assert!(!origin_ok(&headers(&[(header::HOST, "192.168.1.20:47621")])), "no origin, no change");
    }

    #[test]
    fn wrong_codes_lock_the_pad_for_a_while() {
        let (tx, _rx) = futures_channel::mpsc::unbounded();
        let mut s = State { pin: "123456".into(), tokens: HashSet::new(), misses: 0, locked_until: None, stop: None, input: tx, run: 0 };
        let t0 = Instant::now();
        for _ in 0..TRIES {
            assert_eq!(check_code(&mut s, "000000", t0), Code::Wrong);
        }
        assert_eq!(check_code(&mut s, "123456", t0), Code::Wait, "even the right code waits");
        assert_eq!(check_code(&mut s, "123456", t0 + LOCKOUT), Code::Right);
    }

    #[test]
    fn buttons_follow_what_is_in_front() {
        assert_eq!(route_key("up", true, false), Some(InputEvent::Move(Dir::Up)));
        assert_eq!(route_key("a", true, false), Some(InputEvent::Press(Pad::A)));
        assert_eq!(route_key("a", false, false), None, "over a game, A goes to the game");
        assert_eq!(outside_key("a"), Some(VK_RETURN));
        assert_eq!(route_key("a", false, true), Some(InputEvent::Press(Pad::A)), "unless our keyboard is up");
        assert_eq!(route_key("home", false, false), Some(InputEvent::Summon));
        assert_eq!(route_key("launch-everything", true, false), None);
    }
}
