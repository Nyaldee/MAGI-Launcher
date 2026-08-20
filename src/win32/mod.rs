//! Fine couche de ré-export par-dessus `windows-sys` (bindings Win32 de
//! Microsoft, générés depuis les métadonnées Windows : déclarations FFI
//! pures, aucune logique ajoutée). Organisée par DLL
//! (user32/kernel32/shell32/dwmapi/gdi32/advapi32/combase) plutôt que par
//! thème, contrairement à windows-sys, pour que chaque site d'appel de core/
//! et ui/ désigne l'API par la bibliothèque qui l'exporte réellement.
//!
//! Les quelques helpers définis ici (chaînes larges, presse-papier) sont la
//! seule logique du module ; tout le reste n'est que ré-export.
// dead_code/unused_imports : quelques ré-exports ne servent que sous
// cfg(test) ou à un seul appelant conditionnel, sans que leur absence
// d'usage direct soit une erreur.
#![allow(non_camel_case_types, non_snake_case, dead_code, unused_imports)]

pub mod advapi32;
pub mod combase;
pub mod dwmapi;
pub mod gdi32;
pub mod kernel32;
pub mod shell32;
pub mod user32;

pub use windows_sys::core::{BOOL, GUID};
pub use windows_sys::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
pub use windows_sys::Win32::UI::WindowsAndMessaging::MSG;

pub type UINT = u32;

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

/// Longueur d'une chaîne UTF-16 dont on n'a que le pointeur, typiquement un
/// buffer rendu par une API Win32/COM sans taille associée.
///
/// # Safety
/// `ptr` doit être valide pour lecture jusqu'à son premier NUL inclus.
pub unsafe fn wstrlen(ptr: *const u16) -> usize {
    let mut len = 0isize;
    while *ptr.offset(len) != 0 {
        len += 1;
    }
    len as usize
}

/// Copie `s` en UTF-16 dans un buffer de taille fixe -- typiquement un champ
/// `[u16; N]` de struct Win32 comme szTip -- en tronquant si nécessaire pour
/// que le NUL terminal tienne toujours.
pub fn copy_into_wide(s: &str, buf: &mut [u16]) {
    let wide = to_wstring(s);
    let n = wide.len().min(buf.len().saturating_sub(1));
    buf[..n].copy_from_slice(&wide[..n]);
    buf[n] = 0;
}

/// `GetLastError()` sans le `unsafe` à chaque site d'appel.
pub fn last_error() -> u32 {
    unsafe { kernel32::GetLastError() }
}

/// Pose `text` dans le presse-papier (CF_UNICODETEXT). `false` si une étape
/// échoue -- presse-papier tenu par un autre process, allocation refusée.
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
        // SetClipboardData réussi, le système devient propriétaire de hmem :
        // le libérer ici (GlobalFree) provoquerait un double free.
        let ok = !user32::SetClipboardData(user32::CF_UNICODETEXT, hmem).is_null();
        user32::CloseClipboard();
        ok
    }
}

/// Lit le presse-papier (CF_UNICODETEXT), symétrique de
/// `set_clipboard_text`. `None` si une étape échoue -- presse-papier vide ou
/// tenu par un autre process, contenu qui n'est pas du texte.
pub fn get_clipboard_text(hwnd: HWND) -> Option<String> {
    unsafe {
        if user32::OpenClipboard(hwnd) == 0 {
            return None;
        }
        let handle = user32::GetClipboardData(user32::CF_UNICODETEXT);
        if handle.is_null() {
            user32::CloseClipboard();
            return None;
        }
        let ptr = kernel32::GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            user32::CloseClipboard();
            return None;
        }
        // Le bloc global n'expose pas sa longueur : comme toute chaîne
        // CF_UNICODETEXT il est terminé par NUL, donc scanné jusque-là.
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, wstrlen(ptr)));
        kernel32::GlobalUnlock(handle);
        user32::CloseClipboard();
        Some(text)
    }
}

/// `true` si le contenu actuel du presse-papier porte le format
/// "ExcludeClipboardContentFromMonitorProcessing" -- convention de facto
/// (pas une API dédiée, juste un format enregistré sous ce nom précis)
/// respectée par Windows lui-même (Clipboard History, Win+V) et par la
/// plupart des gestionnaires de mots de passe (1Password, Bitwarden,
/// KeePass) pour signaler "ne capture pas ceci" à tout logiciel qui
/// surveille le presse-papier. À vérifier AVANT toute lecture dans
/// core::clipboard_history (voir WM_CLIPBOARDUPDATE dans ui::window) : sans
/// ce contrôle, un mot de passe copié depuis une de ces applications
/// atterrit en clair dans l'historique, contre l'intention de la source.
pub fn clipboard_excluded_from_history() -> bool {
    unsafe {
        let format_name = to_wstring("ExcludeClipboardContentFromMonitorProcessing");
        let format = user32::RegisterClipboardFormatW(format_name.as_ptr());
        format != 0 && user32::IsClipboardFormatAvailable(format) != 0
    }
}
