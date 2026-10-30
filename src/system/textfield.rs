//! Telling whether the user just picked a place to type, using UI Automation
//! (what screen readers use), so it works in browsers and most apps alike.

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationValuePattern, UIA_ComboBoxControlTypeId, UIA_DocumentControlTypeId,
    UIA_EditControlTypeId, UIA_ValuePatternId,
};

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, SyncSender, channel, sync_channel};
use std::time::Duration;

/// One UI Automation thread for the whole run. A hung app can stall it, so askers wait only briefly.
static ASK: OnceLock<SyncSender<Sender<bool>>> = OnceLock::new();

/// True when keyboard focus (in another app) is on something you can type into.
pub fn focused() -> bool {
    let ask = ASK.get_or_init(|| {
        let (tx, rx) = sync_channel::<Sender<bool>>(1);
        let _ = std::thread::Builder::new().name("text-field".into()).spawn(move || {
            let uia = unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
            };
            for reply in rx {
                let _ = reply.send(uia.as_ref().is_some_and(check));
            }
        });
        tx
    });
    let (reply, answer) = channel();
    // A full queue means the thread is still stuck on the last question.
    if ask.try_send(reply).is_err() {
        return false;
    }
    answer.recv_timeout(Duration::from_millis(500)).unwrap_or(false)
}

fn check(uia: &IUIAutomation) -> bool {
    unsafe {
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
