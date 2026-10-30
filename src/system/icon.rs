//! App icons, read from the program or shortcut and saved as PNG so the app
//! chips can show the real artwork instead of an initial.

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use image::{ImageFormat, RgbaImage};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits, GetObjectW, HGDIOBJ,
    ReleaseDC,
};
use sha2::{Digest, Sha256};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Controls::{ILD_TRANSPARENT, IImageList};
use windows::Win32::UI::Shell::{
    SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SYSICONINDEX, SHGetFileInfoW, SHGetImageList, SHIL_JUMBO,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::HSTRING;

/// Below this, a "jumbo" icon is really a small one drawn in a corner, so the normal size looks better.
const SMALL: u32 = 48;

/// The PNG for an app's icon, cached on disk. `None` if the program has no icon.
pub fn png_for(path: &Path) -> Option<PathBuf> {
    let dir = crate::settings::data_root()?.join("icons");
    std::fs::create_dir_all(&dir).ok()?;
    // Hashed, so long paths can't make a file name Windows refuses.
    let name: String = Sha256::digest(path.to_string_lossy().as_bytes()).iter().take(12).map(|b| format!("{b:02x}")).collect();
    let file = dir.join(format!("{name}.png"));
    if !file.exists() {
        let rgba = read_icon(path)?;
        rgba.save_with_format(&file, ImageFormat::Png).ok()?;
    }
    Some(file)
}

fn read_icon(path: &Path) -> Option<RgbaImage> {
    unsafe {
        // Shortcuts are resolved through COM; scan threads haven't set it up.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let path = HSTRING::from(path.as_os_str());
        let size = std::mem::size_of::<SHFILEINFOW>() as u32;
        let mut info = SHFILEINFOW::default();
        // 256 px from the system image list, so icons stay sharp on a TV.
        if SHGetFileInfoW(&path, FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut info), size, SHGFI_SYSICONINDEX) != 0
            && let Ok(list) = SHGetImageList::<IImageList>(SHIL_JUMBO as i32)
            && let Ok(icon) = list.GetIcon(info.iIcon, ILD_TRANSPARENT.0)
        {
            let rgba = icon_pixels(icon).and_then(trim);
            let _ = DestroyIcon(icon);
            if rgba.is_some() {
                return rgba;
            }
        }
        let mut info = SHFILEINFOW::default();
        let ok = SHGetFileInfoW(&path, FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut info), size, SHGFI_ICON | SHGFI_LARGEICON);
        if ok == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let icon = info.hIcon;
        let rgba = icon_pixels(icon);
        let _ = DestroyIcon(icon);
        rgba
    }
}

/// Cuts the empty border off a jumbo icon. `None` when what's left is small.
fn trim(img: RgbaImage) -> Option<RgbaImage> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p.0[3] > 8 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    if x0 > x1 || x1 - x0 < SMALL || y1 - y0 < SMALL {
        return None;
    }
    // Square, centred on what's drawn, so the chip shows it evenly.
    let side = (x1 - x0).max(y1 - y0) + 1;
    let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
    let x = cx.saturating_sub(side / 2).min(img.width() - side.min(img.width()));
    let y = cy.saturating_sub(side / 2).min(img.height() - side.min(img.height()));
    Some(image::imageops::crop_imm(&img, x, y, side.min(img.width()), side.min(img.height())).to_image())
}

unsafe fn icon_pixels(icon: HICON) -> Option<RgbaImage> {
    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(icon, &mut info).ok()?;
        let color = info.hbmColor;
        let mask = info.hbmMask;
        let result = (|| {
            if color.is_invalid() {
                return None;
            }
            let mut bitmap = BITMAP::default();
            let read = GetObjectW(HGDIOBJ(color.0), std::mem::size_of::<BITMAP>() as i32, Some((&mut bitmap as *mut BITMAP).cast()));
            if read == 0 {
                return None;
            }
            let (w, h) = (bitmap.bmWidth.max(1), bitmap.bmHeight.max(1));

            let header = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative height asks for rows top to bottom.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };
            let mut bmi = BITMAPINFO { bmiHeader: header, ..Default::default() };
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            let dc = GetDC(None);
            let rows = GetDIBits(dc, color, 0, h as u32, Some(pixels.as_mut_ptr() as *mut c_void), &mut bmi, DIB_RGB_COLORS);
            ReleaseDC(None, dc);
            if rows == 0 {
                return None;
            }
            // Windows gives BGRA. Icons without an alpha channel come back with alpha 0,
            // so treat them as opaque instead of invisible.
            let has_alpha = pixels.chunks_exact(4).any(|p| p[3] != 0);
            for p in pixels.chunks_exact_mut(4) {
                p.swap(0, 2);
                if !has_alpha {
                    p[3] = 255;
                }
            }
            RgbaImage::from_raw(w as u32, h as u32, pixels)
        })();
        let _ = DeleteObject(HGDIOBJ(color.0));
        let _ = DeleteObject(HGDIOBJ(mask.0));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_with_an_icon_gives_a_png() {
        // The test binary itself always has an icon, or at least a default one.
        let exe = std::env::current_exe().unwrap();
        if let Some(file) = png_for(&exe) {
            assert!(file.exists());
            assert_eq!(file.extension().unwrap(), "png");
        }
    }
}
