use windows::Win32::System::Power::SetSuspendState;

use super::run_hidden;

pub fn sleep() {
    unsafe {
        SetSuspendState(false, false, false);
    }
}

pub fn restart() {
    run_hidden("shutdown", &["/r", "/t", "0"]);
}

pub fn turn_off() {
    run_hidden("shutdown", &["/s", "/hybrid", "/t", "0"]);
}
