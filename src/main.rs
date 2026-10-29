#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod input;
mod library;
mod osk;
mod protocol;
mod settings;
mod system;
mod ui;

use dioxus::desktop::tao::window::{Fullscreen, Icon};
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
use image::imageops::FilterType;
use tracing_subscriber::EnvFilter;

const CSS: &str = include_str!("../assets/css/joy.css");
const ICON: &[u8] = include_bytes!("../assets/images/icon.png");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("joyos=info")))
        .init();

    let settings = settings::Settings::load();
    i18n::set_language(settings.language.as_deref().and_then(i18n::index_of).unwrap_or_else(i18n::detect));
    app::apply_to_input(&settings);
    app::hand_over_input(input::start());

    // `--windowed` is handy while working on JoyOS itself.
    let windowed = std::env::args().any(|a| a == "--windowed");
    let mut window = WindowBuilder::new().with_title("JoyOS").with_decorations(windowed).with_window_icon(window_icon());
    window = if windowed {
        window.with_inner_size(LogicalSize::new(1600.0, 900.0))
    } else {
        window.with_fullscreen(Some(Fullscreen::Borderless(None)))
    };

    dioxus::LaunchBuilder::desktop().with_cfg(config(window, (11, 11, 13, 255), "")).launch(app::App);
}

/// The title bar and taskbar icon. The .exe gets the same artwork from build.rs.
fn window_icon() -> Option<Icon> {
    let rgba = image::load_from_memory(ICON).ok()?.resize(256, 256, FilterType::Lanczos3).into_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

/// Shared by every JoyOS window. Windows in one process share a browser, and
/// WebView2 refuses a second window whose browser settings differ.
pub fn config(window: WindowBuilder, background: (u8, u8, u8, u8), head: &str) -> Config {
    let data_dir = directories::ProjectDirs::from("app", "JoyOS", "JoyOS").map(|d| d.data_local_dir().join("webview"));
    // Button sounds play from controller input, which isn't a "user gesture" to the browser.
    let mut browser_args = String::from("--autoplay-policy=no-user-gesture-required");
    if cfg!(debug_assertions) {
        // Lets dev tools (and tools/cdp.mjs) attach to the page in debug builds.
        browser_args.push_str(" --remote-debugging-port=9333");
    }
    let mut config = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_disable_context_menu(true)
        .with_background_color(background)
        .with_custom_head(format!("<style>{CSS}</style>{head}"))
        .with_custom_protocol("joy", protocol::handle)
        .with_windows_browser_args(browser_args);
    if let Some(dir) = data_dir {
        config = config.with_data_directory(dir);
    }
    config
}
