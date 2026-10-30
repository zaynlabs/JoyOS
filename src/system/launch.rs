use std::path::Path;

use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// Opens a URI (steam://…), shortcut, or program the same way a double-click would.
pub fn open(target: &str) -> bool {
    run(target, None, None)
}

/// Opens a game. Many games only start from their own folder, so that's where they run.
pub fn open_in(target: &str, params: Option<&str>, folder: Option<&Path>) -> bool {
    run(target, params, folder.filter(|f| f.is_dir()))
}

/// What an emulator is given: the template with `{file}` filled in, or just the file.
pub fn emulator_args(template: &str, file: &str) -> String {
    let quoted = format!("\"{file}\"");
    let template = template.trim();
    if template.is_empty() {
        quoted
    } else if template.contains("{file}") {
        template.replace("\"{file}\"", &quoted).replace("{file}", &quoted)
    } else {
        format!("{template} {quoted}")
    }
}

/// A web address that's safe to hand to the browser: http(s) with a host, and
/// written out in full so nothing in it can pass for an extra option.
pub fn web_link(text: &str) -> Option<String> {
    let url = url::Url::parse(text.trim()).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host().is_some()).then(|| url.to_string())
}

/// Opens a web page full screen in Edge, so it reads like an app and not a browser.
pub fn open_kiosk(url: &str) -> bool {
    match web_link(url) {
        Some(url) => run("msedge.exe", Some(&format!("--kiosk \"{url}\" --edge-kiosk-type=fullscreen")), None),
        None => false,
    }
}

fn run(program: &str, params: Option<&str>, folder: Option<&Path>) -> bool {
    let params = params.map(HSTRING::from);
    let params = params.as_ref().map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr()));
    let folder = folder.map(|f| HSTRING::from(f.as_os_str()));
    let folder = folder.as_ref().map_or(PCWSTR::null(), |f| PCWSTR(f.as_ptr()));
    let result = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(program), params, folder, SW_SHOWNORMAL) };
    // ShellExecute reports success as any value above 32.
    let ok = result.0 as usize > 32;
    if !ok {
        tracing::warn!("couldn't open {program} (code {})", result.0 as usize);
    }
    ok
}

pub fn show_folder(path: &Path) -> bool {
    open(&path.to_string_lossy())
}

/// Shows or hides the Windows touch keyboard over whatever app is in front.
pub fn touch_keyboard() {
    if !tip::toggle() {
        open("osk.exe");
    }
}

/// Like `touch_keyboard`, but never hides it: picking a second text box
/// shouldn't close the keyboard the first one opened.
pub fn show_touch_keyboard() {
    if !tip::visible() {
        touch_keyboard();
    }
}

#[allow(non_snake_case)]
mod tip {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
    use windows::Win32::System::Com::{CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GWL_STYLE, GetDesktopWindow, GetWindowLongPtrW, IsWindowVisible, WS_DISABLED,
    };
    use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, PCWSTR, interface, w};

    /// Undocumented but stable since Windows 8: the touch keyboard's toggle.
    #[interface("37c994e7-432b-4834-a2f7-dce1f13b834b")]
    unsafe trait ITipInvocation: IUnknown {
        fn Toggle(&self, wnd: HWND) -> HRESULT;
    }

    const UI_HOST_NO_LAUNCH: GUID = GUID::from_u128(0x4ce576fa_83dc_4f88_951c_9d0782b4e376);

    pub fn toggle() -> bool {
        unsafe {
            // This runs on helper threads that haven't set COM up yet.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let tip: Result<ITipInvocation, _> = CoCreateInstance(&UI_HOST_NO_LAUNCH, None, CLSCTX_LOCAL_SERVER);
            match tip {
                Ok(tip) => tip.Toggle(GetDesktopWindow()).is_ok(),
                Err(_) => {
                    // The keyboard host isn't running yet; start it and it opens itself.
                    let common = std::env::var("CommonProgramW6432")
                        .or_else(|_| std::env::var("CommonProgramFiles"))
                        .unwrap_or_else(|_| r"C:\Program Files\Common Files".into());
                    super::open(&format!(r"{common}\microsoft shared\ink\TabTip.exe"))
                }
            }
        }
    }

    pub fn visible() -> bool {
        unsafe {
            // Windows 11 hosts the keyboard in a window that's cloaked, not hidden, when it's away.
            if let Ok(host) = FindWindowW(w!("Windows.UI.Core.CoreWindow"), w!("Microsoft Text Input Application")) {
                let mut cloaked = 0u32;
                let read = DwmGetWindowAttribute(host, DWMWA_CLOAKED, (&mut cloaked as *mut u32).cast(), 4);
                return read.is_ok() && cloaked == 0;
            }
            // Windows 10.
            match FindWindowW(w!("IPTip_Main_Window"), PCWSTR::null()) {
                Ok(tip) => {
                    IsWindowVisible(tip).as_bool() && GetWindowLongPtrW(tip, GWL_STYLE) & WS_DISABLED.0 as isize == 0
                }
                Err(_) => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_links_must_be_plain_web_addresses() {
        assert_eq!(web_link("https://www.netflix.com").as_deref(), Some("https://www.netflix.com/"));
        // Spaces and quotes can't split the address into extra browser options.
        let sneaky = web_link(r#"https://a.com/" --user-data-dir=C:\x"#).unwrap();
        assert!(!sneaky.contains(' ') && !sneaky.contains('"'));
        assert!(web_link("file:///C:/Windows/System32/cmd.exe").is_none());
        assert!(web_link("javascript:alert(1)").is_none());
        assert!(web_link("--remote-debugging-port=1").is_none());
    }

    #[test]
    fn emulator_templates_fill_in_the_file() {
        assert_eq!(emulator_args("", r"D:\Roms\a b.nes"), r#""D:\Roms\a b.nes""#);
        assert_eq!(emulator_args(r"-L cores\nes.dll {file}", "x.nes"), r#"-L cores\nes.dll "x.nes""#);
        assert_eq!(emulator_args(r#"-f "{file}""#, "x.nes"), r#"-f "x.nes""#);
        assert_eq!(emulator_args("--fullscreen", "x.nes"), r#"--fullscreen "x.nes""#);
    }
}
