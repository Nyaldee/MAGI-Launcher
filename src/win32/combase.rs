//! combase.dll -- CoTaskMemFree, pour libérer le buffer renvoyé par
//! SHGetKnownFolderPath (voir core::launch::known_folder). Pas de
//! CoInitialize : cette API-là ne l'exige pas.

pub use windows_sys::Win32::System::Com::CoTaskMemFree;
