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

/// Shows the Windows touch keyboard over whatever app is in front.
pub fn touch_keyboard() {
    if !tip::toggle() {
        open("osk.exe");
    }
}

#[allow(non_snake_case)]
mod tip {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CLSCTX_LOCAL_SERVER, CoCreateInstance};
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
    use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, interface};

    /// Undocumented but stable since Windows 8: the touch keyboard's toggle.
    #[interface("37c994e7-432b-4834-a2f7-dce1f13b834b")]
    unsafe trait ITipInvocation: IUnknown {
        fn Toggle(&self, wnd: HWND) -> HRESULT;
    }

    const UI_HOST_NO_LAUNCH: GUID = GUID::from_u128(0x4ce576fa_83dc_4f88_951c_9d0782b4e376);

    pub fn toggle() -> bool {
        unsafe {
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
}
