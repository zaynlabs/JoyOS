//! Telling whether the user just picked a place to type, using UI Automation
//! (what screen readers use), so it works in browsers and most apps alike.

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationValuePattern, UIA_ComboBoxControlTypeId, UIA_DocumentControlTypeId,
    UIA_EditControlTypeId, UIA_ValuePatternId,
};

/// True when keyboard focus (in another app) is on something you can type into.
pub fn focused() -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(uia) = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER) else {
            return false;
        };
        let Ok(el) = uia.GetFocusedElement() else { return false };
        if !el.CurrentIsEnabled().is_ok_and(|b| b.as_bool())
            || el.CurrentProcessId().is_ok_and(|pid| pid as u32 == std::process::id())
        {
            return false;
        }
        let Ok(kind) = el.CurrentControlType() else { return false };
        let writable = el
            .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
            .ok()
            .map(|v| !v.CurrentIsReadOnly().is_ok_and(|b| b.as_bool()));
        // Web pages and rich editors are "documents"; only count the ones that take text.
        match kind {
            k if k == UIA_EditControlTypeId => writable.unwrap_or(true),
            k if k == UIA_DocumentControlTypeId || k == UIA_ComboBoxControlTypeId => writable.unwrap_or(false),
            _ => false,
        }
    }
}
