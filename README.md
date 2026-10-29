# JoyOS

JoyOS is a full-screen shell for Windows that you run with a controller. Pick up a gamepad and you can play games, open apps, change the volume, take a screenshot, or turn the PC off, all from the couch.

## Run it

```sh
cargo run --release            # full screen, like a console
cargo run -- --windowed        # in a window, handy when you're working on JoyOS
```

You need Windows 10 or 11 with the WebView2 runtime. Windows 11 already has it.

## Controls

| Controller | Keyboard | What it does |
|---|---|---|
| D-pad / left stick | Arrows | Move |
| A | Enter | Play, open, or select |
| B | Esc, Backspace | Go back |
| X | X | More about a game |
| Y | F or / | Search games |
| LB / RB | Q / E, Page Up/Down | Switch tabs |
| LT / RT | | Jump a page |
| Right stick | | Tilt the focused cover |
| Start | M, F1 | Quick menu |
| **Start + View** (or Guide) | | Bring JoyOS back from any game or app |

In other apps, the controller works as a mouse. The left stick moves the pointer (hold LT to slow it down), and the right stick scrolls. A, B and X are left, right and middle click. Y opens the on-screen keyboard, which also opens by itself when you click a text box. The D-pad sends arrow keys, LB and RB go back and forward in the browser, RT presses Enter, View presses Esc, and Start opens the quick menu.

## What's inside

- **It finds your games and apps.** That covers Steam (every library folder, with cover art saved locally and the Steam CDN as a fallback), Epic, and apps from the Start Menu.
- **Home.** The game you're focused on takes over the screen with its logo and art. Below it you'll find "Jump back in," with pinned games first and then the ones you played most recently, followed by your apps.
- **Games.** Everything you own, with filters and a search that uses an on-screen keyboard in your language's layout.
- **Typing in other apps.** A JoyOS keyboard floats over whatever you're using. X deletes, Y is space, LT is Shift, RT is Enter, LB and RB move the cursor, View shows symbols, and B closes it. If you'd rather use the Windows touch keyboard, you can switch to it under Settings → Controller.
- **Quick menu.** Go back to the game you were playing, or close it. Adjust sound and brightness (on laptops and handhelds), turn mouse mode on or off, take a screenshot, open the keyboard, or go to the power options.
- **Power.** Sleep, restart, turn off, or drop to the Windows desktop.
- **13 languages,** including Arabic, which reads right to left. The wording is short and everyday. See [locales/README.md](locales/README.md) for the style guide.
- **First run.** JoyOS says hello in every language. Then you pick yours, test your controller, and it finds your games.

## How it's built

- **Rust and [Dioxus 0.7](https://dioxuslabs.com) on desktop.** The interface is written as Rust components and drawn by WebView2, which gives us the blur, refraction, spring motion and variable fonts the design relies on.
- **[gilrs](https://gitlab.com/gilrs-project/gilrs) on its XInput backend.** Windows' default controller backend only listens while the focused window is JoyOS. JoyOS needs to hear the controller while a game or another app is in front, so it uses XInput instead.
- **The `windows` crate** handles volume (Core Audio), the foreground window, synthetic mouse and keyboard input, ShellExecute, power, and the registry.
- **Fluent** (`fluent-bundle`) handles translations, with each language's plural rules.

| Path | What's there |
|---|---|
| `src/app.rs` | Shell state, input handling, the root component |
| `src/input.rs` | Controller thread: menu moves, mouse mode, rumble |
| `assets/js/nav.js` | Focus engine. It picks the next item from on-screen positions |
| `assets/css/joy.css` | Design tokens, glass, motion, and every screen's styles |
| `src/ui/` | Screens (home, games, apps, settings) and layers (quick menu, power, game page, keyboard, setup) |
| `src/library/` | Steam, Epic and Start Menu discovery, plus accent colours from cover art |
| `src/system/` | Windows integration |
| `tools/cdp.mjs` | Dev helper for screenshots and key presses against a debug build |

Settings are saved in `%APPDATA%\JoyOS\JoyOS\config\settings.toml`.

## Develop

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

Debug builds open a DevTools port on `127.0.0.1:9333`. You can use it to take a screenshot or press keys:

```sh
node tools/cdp.mjs shot out.png ArrowRight Enter
```

Fonts are Inter and Instrument Serif, both under the SIL Open Font License. The license texts are in `assets/fonts/`.
