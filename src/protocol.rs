//! The `joy` protocol serves bundled fonts and local cover art to the webview.
//! Pages reach it as `http://joy.localhost/…`. Files are read on worker threads,
//! so a slow disk never stalls the UI.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};

use dioxus::desktop::wry::{
    RequestAsyncResponder, WebViewId,
    http::{Request, Response, StatusCode, header},
};

use crate::library::decode_media_path;

const INTER: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");
const SERIF: &[u8] = include_bytes!("../assets/fonts/InstrumentSerif-Regular.ttf");
const SERIF_ITALIC: &[u8] = include_bytes!("../assets/fonts/InstrumentSerif-Italic.ttf");

/// Only images leave this handler; it isn't a general file server.
const ALLOWED: &[(&str, &str)] = &[("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png")];
const WORKERS: usize = 4;

type Job = (PathBuf, &'static str, RequestAsyncResponder);

pub fn handle(_: WebViewId, request: Request<Vec<u8>>, responder: RequestAsyncResponder) {
    let path = request.uri().path();
    let font = match path {
        "/font/inter" => Some(INTER),
        "/font/serif" => Some(SERIF),
        "/font/serif-italic" => Some(SERIF_ITALIC),
        _ => None,
    };
    if let Some(font) = font {
        return responder.respond(reply(Some((Cow::Borrowed(font), "font/ttf"))));
    }
    let file = path.strip_prefix("/file/").and_then(decode_media_path).and_then(|file| {
        let ext = file.extension()?.to_string_lossy().to_lowercase();
        let (_, mime) = ALLOWED.iter().find(|(e, _)| *e == ext)?;
        Some((file, *mime))
    });
    match file {
        Some((file, mime)) => {
            if let Err(job) = workers().lock().unwrap_or_else(|e| e.into_inner()).send((file, mime, responder)) {
                job.0.2.respond(reply(None));
            }
        }
        None => responder.respond(reply(None)),
    }
}

/// A few threads that read files and answer the webview.
fn workers() -> &'static Mutex<Sender<Job>> {
    static QUEUE: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        let rx = std::sync::Arc::new(Mutex::new(rx));
        for i in 0..WORKERS {
            let rx = rx.clone();
            let _ = std::thread::Builder::new().name(format!("media-{i}")).spawn(move || {
                loop {
                    let job = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                    let Ok((file, mime, responder)) = job else { return };
                    let body = allowed(&file).then(|| std::fs::read(&file).ok()).flatten();
                    responder.respond(reply(body.map(|b| (Cow::Owned(b), mime))));
                }
            });
        }
        Mutex::new(tx)
    })
}

/// Only files under the folders JoyOS keeps art in: its own, and Steam's cover cache.
/// Network paths are never followed, since opening one sends this PC's sign-in to that server.
fn allowed(file: &Path) -> bool {
    let text = file.to_string_lossy();
    if text.starts_with(r"\\") || text.starts_with("//") {
        return false;
    }
    let Ok(file) = file.canonicalize() else { return false };
    roots().iter().any(|root| file.starts_with(root))
}

/// Worked out on each request: the folders may appear during the run.
fn roots() -> Vec<PathBuf> {
    let data = crate::settings::data_root();
    [data.as_ref().map(|d| d.join("art")), data.as_ref().map(|d| d.join("icons")), crate::library::steam_cache()]
        .into_iter()
        .flatten()
        .filter_map(|root| root.canonicalize().ok())
        .collect()
}

fn reply(found: Option<(Cow<'static, [u8]>, &str)>) -> Response<Cow<'static, [u8]>> {
    let builder = Response::builder().header(header::CACHE_CONTROL, "max-age=86400");
    let response = match found {
        Some((body, mime)) => builder.header(header::CONTENT_TYPE, mime).body(body),
        None => builder.status(StatusCode::NOT_FOUND).body(Cow::Borrowed(&b""[..])),
    };
    response.unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_art_folders_are_served() {
        assert!(!allowed(Path::new(r"\\attacker\share\x.png")));
        assert!(!allowed(Path::new(r"C:\Windows\System32\oobe\info\background.jpg")));
        let Some(art) = crate::settings::data_root().map(|d| d.join("art")) else { return };
        // `..` can't climb out of the art folder.
        assert!(!allowed(&art.join(r"..\..\..\..\Windows\win.ini.png")));
    }
}
