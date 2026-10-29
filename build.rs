use image::imageops::FilterType;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/images/icon.png");

    // Windows wants an .ico for the .exe itself (Explorer, taskbar, shortcuts),
    // so build one from the PNG rather than keeping a second copy of the art.
    let png = image::open("assets/images/icon.png").expect("assets/images/icon.png should decode");
    let ico = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("icon.ico");
    png.resize(256, 256, FilterType::Lanczos3).save_with_format(&ico, image::ImageFormat::Ico).expect("icon.ico should encode");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new().set_icon(ico.to_str().unwrap()).compile().expect("icon should embed in the .exe");
    }
}
