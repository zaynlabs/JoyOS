use super::registry;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "JoyOS";

pub fn is_enabled() -> bool {
    registry::read_string(RUN_KEY, NAME).is_some()
}

pub fn set_enabled(on: bool) {
    if on {
        if let Ok(exe) = std::env::current_exe() {
            registry::write_string(RUN_KEY, NAME, &format!("\"{}\"", exe.display()));
        }
    } else {
        registry::delete(RUN_KEY, NAME);
    }
}
