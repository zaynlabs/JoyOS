//! Local crash logs, written only when the user turns crash reports on. Nothing
//! is ever sent anywhere; the log sits in the data folder for the user to read.

use std::backtrace::Backtrace;

pub fn install(enabled: bool) {
    if !enabled {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write_log(&format!("{info}\n\n{}", Backtrace::force_capture()));
        previous(info);
    }));
}

fn write_log(text: &str) {
    let Some(dir) = crate::settings::data_root().map(|d| d.join("crashes")) else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let stamp = crate::app::now();
    let _ = std::fs::write(dir.join(format!("crash-{stamp}.txt")), format!("JoyOS {}\n{text}\n", env!("CARGO_PKG_VERSION")));
}
