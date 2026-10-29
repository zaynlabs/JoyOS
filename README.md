# JoyOS

JoyOS is a full-screen, controller-first shell for Windows. Pick up a gamepad and you can play games, open apps, change the volume, take a screenshot, or turn the PC off, all from the couch without reaching for a mouse.

## Running it

```sh
cargo run --release       # full screen, like a console
cargo run -- --windowed   # in a window, handy while working on JoyOS
```

### Requirements

- Windows 10 or 11, with the WebView2 runtime. Windows 11 already includes it.
- Rust 1.85 or newer (the project uses the 2024 edition).

## Controls

| Controller | Keyboard | What it does |
|---|---|---|
| D-pad / left stick | Arrow keys | Move around |
| A | Enter | Play, open, or select |
| B | Esc, Backspace | Go back |
| X | X | More about a game |
| Y | F or / | Search games |
| LB / RB | Q / E, Page Up / Page Down | Switch tabs |
| LT / RT | | Jump a page |
| Right stick | | Tilt the focused cover |
| Start | M, F1 | Open the quick menu |
| **Start + View** (or Guide) | | Bring JoyOS back from any game or app |

**Inside apps,** the controller works as a mouse. The left stick moves the pointer (hold LT for finer control), and the right stick scrolls. A, B, and X are left, right, and middle click. Y opens the on-screen keyboard, which also opens by itself when you click a text box. The D-pad sends arrow keys, LB and RB go back and forward in the browser, RT presses Enter, View presses Esc, and Start opens the quick menu.

## What it does

- **Finds your games and apps.** It picks up Steam games from every library folder (with local cover art and a fallback to the Steam CDN), Epic games, and apps from the Start Menu.
- **Home screen.** The focused game fills the screen with its logo and artwork. Below that you get a "Jump back in" row, with pinned games first and then your most recently played, followed by your apps.
- **Games.** Browse everything, filter it, or search with an on-screen keyboard that matches your language's layout.
- **Typing in other apps.** A JoyOS keyboard floats over whatever you're using. X deletes, Y is space, LT is Shift, RT is Enter, LB and RB move the cursor, View shows symbols, and B closes it. If you'd rather use the Windows touch keyboard, switch to it under Settings → Controller.
- **Quick menu.** Return to or close the current game, adjust volume and brightness (on laptops and handhelds), toggle mouse mode, take a screenshot, open the keyboard, or get to the power options.
- **Power.** Sleep, restart, shut down, or drop to the Windows desktop.
- **13 languages,** including right-to-left Arabic. The wording is plain and everyday. See [locales/README.md](locales/README.md) for the style guide.
- **First-run setup.** You'll see "Hello" in every language, then pick your language, test your controller, and let JoyOS find your games.

## How it's built

- **Rust and [Dioxus 0.7](https://dioxuslabs.com) for desktop.** The UI is written as Rust components and rendered by WebView2. That's what makes the Liquid Glass look possible, with real `backdrop-filter` blur, SVG refraction, spring easing, and variable fonts.
- **[gilrs](https://gitlab.com/gilrs-project/gilrs) on the XInput backend.** The default Windows Gaming Input backend only reports controller input while the app has focus. JoyOS needs to keep hearing the controller while a game or app is in front.
- **The `windows` crate** handles volume (Core Audio), the foreground window, synthetic mouse and keyboard input, ShellExecute, power, and the registry.
- **Fluent** (`fluent-bundle`) provides the translations, with plural rules for each language.

Here's where things live:

| Path | What's in it |
|---|---|
| `src/app.rs` | Shell state, input handling, root component |
| `src/input.rs` | Controller thread: menu movement, mouse mode, rumble |
| `src/osk.rs` | The on-screen keyboard window that floats over other apps |
| `assets/js/nav.js` | Spatial focus engine that picks the next item from on-screen positions |
| `assets/css/joy.css` | Design tokens, glass effects, motion, and every screen's styles |
| `src/ui/` | Screens (home, games, apps, settings) and layers (quick menu, power, game page, keyboard, setup) |
| `src/library/` | Steam, Epic, and Start Menu discovery, plus accent colors pulled from cover art |
| `src/system/` | Windows integration |

Your settings are saved to `%APPDATA%\JoyOS\JoyOS\config\settings.toml`.

## Developing

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

Debug builds open a DevTools port on `127.0.0.1:9333`. Point a Chromium-based DevTools client (such as Edge's `edge://inspect`) at it to inspect the interface.

Fonts: Inter and Instrument Serif, both under the SIL Open Font License. The license files are in `assets/fonts/`.
