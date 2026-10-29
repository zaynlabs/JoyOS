//! The `joy` protocol serves bundled fonts and local cover art to the webview.
//! Pages reach it as `http://joy.localhost/…`.

use std::borrow::Cow;

use dioxus::desktop::wry::{
    WebViewId,
    http::{Request, Response, StatusCode, header},
};

use crate::library::decode_media_path;

const INTER: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");
const SERIF: &[u8] = include_bytes!("../assets/fonts/InstrumentSerif-Regular.ttf");
const SERIF_ITALIC: &[u8] = include_bytes!("../assets/fonts/InstrumentSerif-Italic.ttf");

/// Only images leave this handler; it isn't a general file server.
const ALLOWED: &[(&str, &str)] = &[("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png")];

pub fn handle(_: WebViewId, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    let found: Option<(Cow<'static, [u8]>, &str)> = match path {
        "/font/inter" => Some((Cow::Borrowed(INTER), "font/ttf")),
        "/font/serif" => Some((Cow::Borrowed(SERIF), "font/ttf")),
        "/font/serif-italic" => Some((Cow::Borrowed(SERIF_ITALIC), "font/ttf")),
        _ => path.strip_prefix("/file/").and_then(|encoded| {
            let file = decode_media_path(encoded)?;
            let ext = file.extension()?.to_string_lossy().to_lowercase();
            let (_, mime) = ALLOWED.iter().find(|(e, _)| *e == ext)?;
            Some((Cow::Owned(std::fs::read(&file).ok()?), *mime))
        }),
    };

    let builder = Response::builder()
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, "max-age=86400");
    match found {
        Some((body, mime)) => builder.header(header::CONTENT_TYPE, mime).body(body),
        None => builder.status(StatusCode::NOT_FOUND).body(Cow::Borrowed(&b""[..])),
    }
    .expect("static response parts are valid")
}
