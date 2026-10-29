//! Talking to Windows: windows, sound, power, input injection, and so on.
//! Everything here fails quietly — a missing feature should never crash the shell.

pub mod audio;
pub mod display;
pub mod inject;
pub mod launch;
pub mod power;
pub mod registry;
pub mod startup;
pub mod status;
pub mod textfield;
pub mod window;

use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Runs a console tool without flashing a console window, returning its stdout.
pub fn run_hidden(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs blocking work on a thread and awaits it, keeping the UI thread free.
pub async fn off_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = futures_channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    rx.await.ok()
}
