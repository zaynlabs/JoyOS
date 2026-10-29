# Writing for JoyOS

Each folder holds one language. `en/main.ftl` is the source; every other file must have the same keys (`cargo test` checks this).

How JoyOS talks:

- **Short and plain.** "Play", "Sound", "Turn off". Not "Launch", "Audio output", "Shut down system".
- **No tech words.** No "executable", "library", "endpoint", "cache", "sync", "config".
- **Sentence case.** "Look for new games", not "Look For New Games".
- **Talk to one person**, warmly and informally where the language allows it (tu / du / 你).
- **Say what happens, not how.** "Open JoyOS when the PC starts", not "Run at login".

Add a language by creating `locales/<code>/main.ftl` and one line in `LANGUAGES` in `src/i18n.rs`.
