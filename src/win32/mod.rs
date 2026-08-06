//! Fine couche de ré-export par-dessus `windows-sys` (bindings Win32
//! officiels de Microsoft -- déclarations FFI pures, aucune logique
//! ajoutée, générées depuis les métadonnées Windows elles-mêmes). Garde la
//! même organisation par DLL (user32/kernel32/shell32/dwmapi/gdi32) que
//! l'ancien FFI écrit à la main qu'elle remplace, pour que le reste du
//! projet (core/, ui/) n'ait pas besoin d'être retouché en profondeur.
// unused_imports/dead_code : cette couche ré-exporte volontairement toute
// la surface Win32 inventoriée pour le projet (voir le plan de portage),
// pas seulement ce que les modules actuels utilisent déjà -- une bonne
// partie sert de réserve prête à l'emploi pour les prochaines évolutions,
// au même titre qu'un fichier d'en-têtes C.
#![allow(non_camel_case_types, non_snake_case, dead_code, unused_imports)]

pub mod advapi32;
pub mod dwmapi;
pub mod gdi32;
pub mod kernel32;
pub mod shell32;
pub mod user32;

pub use windows_sys::core::BOOL;
pub use windows_sys::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
pub use windows_sys::Win32::UI::WindowsAndMessaging::MSG;

pub type DWORD = u32;
pub type WORD = u16;
pub type BYTE = u8;
pub type UINT = u32;
pub type WCHAR = u16;
pub type ATOM = u16;
pub type COLORREF = u32;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

/// UTF-16, terminé par NUL -- la forme attendue par toute API Win32 en
/// `...W` pour une chaîne en entrée.
pub fn to_wstring(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Décode un buffer UTF-16 (terminé par NUL ou entièrement rempli) en
/// String, en s'arrêtant au premier NUL s'il y en a un.
pub fn from_wstring(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Copie `s` (UTF-16, tronqué si besoin) dans un buffer de taille fixe
/// (typiquement un champ `[u16; N]` de struct Win32 comme szTip), toujours
/// terminé par NUL -- partagé par tout ce qui doit remplir ce genre de
/// champ (tray, futur menu/liste).
pub fn copy_into_wide(s: &str, buf: &mut [u16]) {
    let wide = to_wstring(s);
    let n = wide.len().min(buf.len().saturating_sub(1));
    buf[..n].copy_from_slice(&wide[..n]);
    buf[n] = 0;
}

/// Pour que les sites d'appel lisent l'intention ("qu'a dit
/// GetLastError()") plutôt qu'un appel kernel32 nu.
pub fn last_error() -> u32 {
    unsafe { kernel32::GetLastError() }
}

/// Pose `text` dans le presse-papier (format CF_UNICODETEXT) -- utilisé
/// par le mode calculatrice/aperçu couleur et par Sticky Notes. `false` en
/// cas d'échec à n'importe quelle étape (presse-papier déjà tenu par un
/// autre process, mémoire...).
pub fn set_clipboard_text(hwnd: HWND, text: &str) -> bool {
    let wide = to_wstring(text);
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    unsafe {
        if user32::OpenClipboard(hwnd) == 0 {
            return false;
        }
        user32::EmptyClipboard();
        let hmem = kernel32::GlobalAlloc(kernel32::GMEM_MOVEABLE, byte_len);
        if hmem.is_null() {
            user32::CloseClipboard();
            return false;
        }
        let ptr = kernel32::GlobalLock(hmem) as *mut u16;
        if ptr.is_null() {
            user32::CloseClipboard();
            return false;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        kernel32::GlobalUnlock(hmem);
        // Une fois SetClipboardData réussi, le système possède hmem -- ne
        // jamais le libérer nous-mêmes (GlobalFree) dans ce cas, sous
        // peine de double-free.
        let ok = !user32::SetClipboardData(user32::CF_UNICODETEXT, hmem).is_null();
        user32::CloseClipboard();
        ok
    }
}
