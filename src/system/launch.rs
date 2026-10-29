use std::path::Path;

use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// Opens a URI (steam://…), shortcut, or program the same way a double-click would.
pub fn open(target: &str) -> bool {
    let result = unsafe {
        ShellExecuteW(None, w!("open"), &HSTRING::from(target), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL)
    };
    // ShellExecute reports success as any value above 32.
    result.0 as usize > 32
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
                    super::open(r"C:\Program Files\Common Files\microsoft shared\ink\TabTip.exe")
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
