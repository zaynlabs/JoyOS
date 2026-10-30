//! Speaking instead of typing, for JoyOS's own search. Windows does the listening
//! (the same speech service as Win+H), so it needs the microphone and Windows'
//! online speech setting. The web view can't do this itself: WebView2 has no
//! working speech recognition.

use windows::Foundation::TimeSpan;
use windows::Globalization::Language;
use windows::Media::SpeechRecognition::{SpeechRecognitionResultStatus, SpeechRecognizer};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::HSTRING;

/// 100-nanosecond ticks, the unit Windows times use.
const fn seconds(s: i64) -> TimeSpan {
    TimeSpan { Duration: s * 10_000_000 }
}

/// Listens once and returns what was said. `None` if speech isn't available or nothing was heard.
/// Blocks for up to about ten seconds; run it off the UI thread.
pub fn listen(language: &str) -> Option<String> {
    let result = (|| -> windows::core::Result<Option<String>> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        // Fall back to Windows' own language if ours has no speech pack.
        let recognizer = Language::CreateLanguage(&HSTRING::from(language))
            .and_then(|lang| SpeechRecognizer::Create(&lang))
            .or_else(|_| SpeechRecognizer::new())?;
        let timeouts = recognizer.Timeouts()?;
        timeouts.SetInitialSilenceTimeout(seconds(5))?;
        timeouts.SetBabbleTimeout(seconds(8))?;
        timeouts.SetEndSilenceTimeout(TimeSpan { Duration: 12_000_000 })?;
        recognizer.CompileConstraintsAsync()?.join()?;
        let heard = recognizer.RecognizeAsync()?.join()?;
        let text = heard.Text()?.to_string_lossy();
        Ok((heard.Status()? == SpeechRecognitionResultStatus::Success && !text.trim().is_empty()).then_some(text))
    })();
    result.unwrap_or_else(|e| {
        tracing::info!("voice isn't available: {e}");
        None
    })
}
