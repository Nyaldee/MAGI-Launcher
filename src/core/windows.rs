//! Énumération/activation/fermeture/kill des fenêtres pour le Window
//! Switcher.
//!
//! Les handles (`HWND`) sont de vrais pointeurs (`*mut c_void`) fournis
//! par windows-sys, pas des `isize` -- comparer à `0` ne compile pas, il
//! faut `.is_null()`.

use crate::win32::dwmapi::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use crate::win32::kernel32::{CloseHandle, OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use crate::win32::user32::{
    GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, EnumWindows, GW_OWNER, GWL_EXSTYLE,
    SW_RESTORE, WM_CLOSE, WS_EX_TOOLWINDOW,
};
use crate::win32::{from_wstring, BOOL, HWND, LPARAM};

#[derive(Debug, Clone)]
pub struct WindowEntry {
    pub hwnd: HWND,
    pub title: String,
}

// WindowEntry transporte un pointeur brut, donc n'est ni Send ni Sync :
// invariant à conserver, elle reste cantonnée au thread UI.

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam as *mut Vec<WindowEntry>);
    if is_switcher_candidate(hwnd) {
        if let Some(title) = window_title(hwnd) {
            out.push(WindowEntry { hwnd, title });
        }
    }
    crate::win32::TRUE
}

/// Approximation du filtrage d'Alt+Tab (pas une réimplémentation exacte de
/// `GetLastActivePopup`) : visible, sans fenêtre propriétaire, pas un
/// WS_EX_TOOLWINDOW, et pas "cloaked" par DWM -- ce dernier test écarte les
/// fenêtres fantômes UWP/ApplicationFrameHost, invisibles mais qui passent
/// quand même IsWindowVisible.
fn is_switcher_candidate(hwnd: HWND) -> bool {
    unsafe {
        if IsWindowVisible(hwnd) == 0 {
            return false;
        }
        if !GetWindow(hwnd, GW_OWNER).is_null() {
            return false;
        }
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            &mut cloaked as *mut u32 as *mut core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        if hr == 0 && cloaked != 0 {
            return false;
        }
        true
    }
}

fn window_title(hwnd: HWND) -> Option<String> {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return None;
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if copied <= 0 {
            return None;
        }
        let s = from_wstring(&buf);
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

pub fn list_windows() -> Vec<WindowEntry> {
    let mut out: Vec<WindowEntry> = Vec::new();
    unsafe {
        EnumWindows(Some(enum_proc), &mut out as *mut Vec<WindowEntry> as LPARAM);
    }
    out
}

pub fn activate_window(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }
}

/// Fermeture polie (WM_CLOSE, comme un clic sur la croix) -- laisse
/// l'appli proposer "enregistrer les modifications ?" si elle le souhaite.
pub fn close_window(hwnd: HWND) {
    unsafe {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

/// Dernier recours pour une fenêtre qui ne répond plus à `close_window`.
/// Action délibérément séparée et plus difficile à déclencher (Maj+Suppr
/// dans ui::window) : beaucoup de fenêtres, celles de l'Explorateur en
/// particulier, partagent le processus du bureau et de la barre des tâches
/// -- TerminateProcess y fait tomber tout explorer.exe, pas seulement la
/// fenêtre visée.
pub fn kill_window(hwnd: HWND) {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return;
        }
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !handle.is_null() {
            TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}
