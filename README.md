# JoyOS

A full-screen, controller-first shell for Windows PCs. Pick up a gamepad and use the whole PC from the couch: play games, open apps, change the sound, take screenshots, and turn the PC off without touching a mouse.

## Run it

```sh
cargo run --release            # full screen, like a console
cargo run -- --windowed        # in a window, for working on JoyOS
```

Needs Windows 10/11 with the WebView2 runtime (built into Windows 11).

## Controls

| Controller | Keyboard | What it does |
|---|---|---|
| D-pad / left stick | Arrows | Move |
| A | Enter | Play / open / select |
| B | Esc, Backspace | Back |
| X | X | More about a game |
| Y | F or / | Search games |
| LB / RB | Q / E, Page Up/Down | Switch tabs |
| LT / RT | | Jump a page |
| Right stick | | Tilt the focused cover |
| Start | M, F1 | Quick menu |
| **Start + View** (or Guide) | | Bring JoyOS back from any game or app |

**In apps (controller as mouse):** left stick moves the pointer (hold LT for precision), right stick scrolls, A/B/X are left/right/middle click, Y opens the touch keyboard, D-pad sends arrow keys, LB/RB are browser back and forward, RT is Enter, View is Esc, and Start opens the quick menu.

## What's inside

- **Finds your stuff by itself:** Steam games (all library folders, with local cover art and a fallback to the Steam CDN), Epic games, and apps from the Start Menu.
- **Home:** the focused game fills the screen, using its logo and art. Below it are a "Jump back in" row (pinned games first, then the most recently played) and your apps.
- **Games:** every game, with filters and search through an on-screen keyboard that follows your language's layout.
- **Quick menu:** go back to or close the game you were in, adjust sound and brightness (on laptops and handhelds), turn mouse mode on or off, take a screenshot, open the touch keyboard, and get to power options.
- **Power:** sleep, restart, turn off, or go to the Windows desktop.
- **13 languages**, including right-to-left Arabic, with plain everyday wording. See [locales/README.md](locales/README.md).
- **First-run setup:** "Hello" in every language, then you pick a language, test your controller, and JoyOS finds your games.

## How it's built

- **Rust + [Dioxus 0.7](https://dioxuslabs.com) desktop.** The UI is Rust components, rendered by WebView2 so the Liquid Glass look can use real `backdrop-filter` blur, SVG refraction, spring easing and variable fonts.
- **[gilrs](https://gitlab.com/gilrs-project/gilrs) on its XInput backend.** The default Windows Gaming Input backend only gets input while JoyOS is focused, and JoyOS has to hear the controller while a game or app is in front.
- **`windows` crate** for volume (Core Audio), the foreground window, synthetic mouse and keyboard input, ShellExecute, power, and the registry.
- **Fluent** (`fluent-bundle`) for translations, with proper plural rules per language.

| Path | What |
|---|---|
| `src/app.rs` | Shell state, input handling, root component |
| `src/input.rs` | Controller thread: menu moves, mouse mode, rumble |
| `assets/js/nav.js` | Spatial focus engine: finds the next item from on-screen positions |
| `assets/css/joy.css` | Design tokens, glass, motion, every screen |
| `src/ui/` | Screens (home, games, apps, settings) and layers (quick menu, power, game page, keyboard, setup) |
| `src/library/` | Steam, Epic and Start Menu discovery, plus accent colours from cover art |
| `src/system/` | Windows integration |
| `tools/cdp.mjs` | Dev helper: screenshots and scripted key presses against a debug build |

Settings live in `%APPDATA%\JoyOS\JoyOS\config\settings.toml`.

## Develop

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

Debug builds open a DevTools port on `127.0.0.1:9333`:

```sh
node tools/cdp.mjs shot out.png ArrowRight Enter
```

Fonts: Inter and Instrument Serif, both under the SIL Open Font License (licenses are in `assets/fonts/`).
