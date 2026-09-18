//! dwmapi.dll -- juste de quoi détecter les fenêtres "cloaked" par DWM
//! (fenêtres fantômes UWP/ApplicationFrameHost invisibles qui passent
//! quand même IsWindowVisible) lors de l'énumération du Window Switcher.

pub use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
