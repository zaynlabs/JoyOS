//! Talking to Windows: windows, sound, power, input injection, and so on.
//! Everything here fails quietly — a missing feature should never crash the shell.

pub mod audio;
pub mod display;
pub mod icon;
pub mod inject;
pub mod launch;
pub mod power;
pub mod random;
pub mod registry;
pub mod startup;
pub mod status;
pub mod textfield;
pub mod voice;
pub mod window;

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Console tools that take longer than this are stuck; we stop waiting and end them.
const TOOL_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs a console tool without flashing a console window, returning its stdout.
pub fn run_hidden(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // Read on threads so a chatty tool can't fill its pipe and stall.
    let mut pipes = (child.stdout.take()?, child.stderr.take()?);
    let out = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = pipes.0.read_to_string(&mut text);
        text
    });
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = pipes.1.read_to_string(&mut text);
        text
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < TOOL_TIMEOUT => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                tracing::warn!("{program} took too long; stopped it");
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let (out, err) = (out.join().unwrap_or_default(), err.join().unwrap_or_default());
    if !status.success() {
        tracing::debug!("{program} failed: {}", err.trim());
    }
    status.success().then_some(out)
}

/// Runs blocking work on a thread and awaits it, keeping the UI thread free.
pub async fn off_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = futures_channel::oneshot::channel();
    std::thread::spawn(move || match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
        Ok(value) => {
            let _ = tx.send(value);
        }
        Err(e) => {
            let why = e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned());
            tracing::error!("background work failed: {}", why.unwrap_or_default());
        }
    });
    rx.await.ok()
}
