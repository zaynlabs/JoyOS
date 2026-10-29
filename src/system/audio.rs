use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx};

fn speakers() -> windows::core::Result<IAudioEndpointVolume> {
    unsafe {
        // Already initialised on the UI thread; harmless to ask again.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        devices.GetDefaultAudioEndpoint(eRender, eConsole)?.Activate(CLSCTX_ALL, None)
    }
}

/// 0.0 – 1.0
pub fn volume() -> Option<f32> {
    unsafe { speakers().and_then(|s| s.GetMasterVolumeLevelScalar()).ok() }
}

pub fn set_volume(level: f32) {
    if let Ok(s) = speakers() {
        unsafe {
            let _ = s.SetMute(false, std::ptr::null());
            let _ = s.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null());
        }
    }
}
