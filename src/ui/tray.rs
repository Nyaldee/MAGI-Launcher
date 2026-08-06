//! Icône dans la zone de notification (Shell_NotifyIconW), sans aucune
//! dépendance tierce. Le menu contextuel n'est PAS géré ici -- voir
//! ui::popup_menu.
//!
//! Le clic sur l'icône ne remontait jamais (enregistrement
//! Shell_NotifyIconW pourtant confirmé réussi), quelle que soit la fenêtre
//! testée (la fenêtre de recherche elle-même, une fenêtre message-only
//! façon HWND_MESSAGE...), jusqu'à reproduire ICI la fenêtre dédiée du
//! crate `tray-icon` de tauri-apps (implémentation Windows vérifiée
//! fonctionnelle) : une vraie fenêtre WS_OVERLAPPED (pas WS_POPUP/
//! HWND_MESSAGE) avec ces styles étendus précis, ET surtout un VRAI WndProc
//! qui traite le clic DANS le WndProc (appelé normalement par
//! DispatchMessageW), plutôt qu'un court-circuit dans la boucle de
//! messages qui lisait wParam/lParam directement sans jamais appeler
//! DispatchMessageW pour ce message -- ce dernier point est ce qui
//! manquait aux tentatives précédentes (fenêtre différente, mais toujours
//! avec ce même court-circuit), donc probablement la vraie cause.

use std::path::Path;

use crate::core::hotkey::GlobalHotkey;
use crate::win32::shell32::{
    Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
};
use crate::win32::user32::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyIcon, GetWindowLongPtrW, LoadIconW,
    LoadImageW, PostQuitMessage, RegisterClassExW, SetWindowLongPtrW, HICON, IDI_APPLICATION, IMAGE_ICON,
    LR_DEFAULTSIZE, LR_LOADFROMFILE, MSGFLT_ALLOW, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_NCCREATE,
    WM_RBUTTONDOWN, WM_RBUTTONUP, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED,
};
use crate::win32::{copy_into_wide, to_wstring, HWND, LPARAM, LRESULT, UINT, WPARAM};
use super::gdi::simple_wndclass;

pub const WM_TRAYICON: u32 = WM_APP + 1;

const WINDOW_CLASS_NAME: &str = "MAGILauncherTrayClass";

pub struct TrayState {
    pub main_hwnd: HWND,
    pub hotkey_spec: String,
    pub hotkey: Option<GlobalHotkey>,
    pub hotkey_enabled: bool,
    notify: NOTIFYICONDATAW,
    /// Appelé sur clic droit pour construire/afficher le menu thémé --
    /// dépend de ui::window/ui::popup_menu, câblé depuis main.rs pour ne
    /// pas faire dépendre ce module du reste de l'appli. Reçoit le hwnd de
    /// LA FENÊTRE TRAY (pas main_hwnd) pour pouvoir ré-enregistrer le
    /// hotkey dessus.
    pub on_right_click: Box<dyn FnMut(HWND, &mut TrayState)>,
}

impl TrayState {
    /// `notify` (détails Shell_NotifyIcon) est posé plus tard par
    /// `create_tray_window`, qui seul connaît le hwnd de la fenêtre tray --
    /// inexistant tant que cette fonction n'a pas construit `state`.
    pub fn new(
        main_hwnd: HWND,
        hotkey_spec: String,
        on_right_click: Box<dyn FnMut(HWND, &mut TrayState)>,
    ) -> TrayState {
        TrayState {
            main_hwnd,
            hotkey_spec,
            hotkey: None,
            hotkey_enabled: false,
            notify: NOTIFYICONDATAW::default(),
            on_right_click,
        }
    }

    pub fn set_tooltip(&mut self, tooltip: &str) {
        copy_into_wide(tooltip, &mut self.notify.szTip);
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &mut self.notify);
        }
    }
}

unsafe extern "system" fn tray_wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Motif du tray_proc du crate tray-icon : le pointeur utilisateur est
    // reçu via lpCreateParams dès WM_NCCREATE (avant la fin de
    // CreateWindowExW), stocké dans GWLP_USERDATA ici plutôt qu'après coup.
    if msg == WM_NCCREATE {
        let createstruct = &*(lparam as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, createstruct.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &mut *ptr;

    if msg == WM_TRAYICON {
        match lparam as u32 {
            WM_LBUTTONUP => super::window::toggle(state.main_hwnd),
            WM_RBUTTONDOWN | WM_RBUTTONUP | WM_CONTEXTMENU => {
                // Échange temporaire pour pouvoir passer &mut state à la
                // closure tout en l'appelant via state.on_right_click.
                let mut cb = std::mem::replace(&mut state.on_right_click, Box::new(|_, _| {}));
                cb(hwnd, state);
                state.on_right_click = cb;
            }
            _ => {}
        }
        return 0;
    }

    if msg == WM_DESTROY {
        remove_icon(&mut state.notify);
        drop(Box::from_raw(ptr));
        PostQuitMessage(0);
        return 0;
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Charge l'icône embarquée dans l'exe lui-même (voir build.rs/winresource,
/// ressource ID 1) -- fiable même si Icon.ico n'est pas à côté de l'exe
/// (ex: exe copié seul ailleurs), contrairement à un chargement depuis le
/// fichier externe qui échouait silencieusement dans ce cas et retombait
/// sur l'icône générique de Windows. `icon_path` reste un repli si jamais
/// la ressource embarquée manquait (build sans winresource, par exemple).
fn load_icon(icon_path: &Path) -> HICON {
    const EMBEDDED_ICON_RESOURCE_ID: *const u16 = 1usize as *const u16;
    unsafe {
        let hinstance = crate::win32::kernel32::GetModuleHandleW(std::ptr::null());
        let handle = LoadIconW(hinstance, EMBEDDED_ICON_RESOURCE_ID);
        if !handle.is_null() {
            return handle;
        }
    }
    if icon_path.exists() {
        let wide = to_wstring(&icon_path.to_string_lossy());
        let handle = unsafe {
            LoadImageW(std::ptr::null_mut(), wide.as_ptr(), IMAGE_ICON, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE)
        };
        if !handle.is_null() {
            return handle as HICON;
        }
    }
    // Repli sur l'icône générique de Windows plutôt qu'une icône vide --
    // une NOTIFYICONDATAW sans hIcon valide ne s'affiche pas dans la zone
    // de notification.
    unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) }
}

unsafe fn add_icon(data: &mut NOTIFYICONDATAW) {
    Shell_NotifyIconW(NIM_ADD, data);
}

unsafe fn remove_icon(data: &mut NOTIFYICONDATAW) {
    Shell_NotifyIconW(NIM_DELETE, data);
    DestroyIcon(data.hIcon);
}

/// Crée la fenêtre invisible dédiée (voir le commentaire en tête de
/// fichier), y ajoute l'icône et enregistre le hotkey global -- tout en un
/// seul appel, `main_hwnd`/`hotkey_spec`/`on_right_click` déjà posés dans
/// `state`, le reste (icône, hotkey) posé ici une fois le hwnd connu.
pub unsafe fn create_tray_window(mut state: Box<TrayState>, icon_path: &Path, tooltip: &str) -> Result<HWND, String> {
    let hinstance = crate::win32::kernel32::GetModuleHandleW(std::ptr::null());
    let class_name = to_wstring(WINDOW_CLASS_NAME);
    let wc = simple_wndclass(class_name.as_ptr(), Some(tray_wndproc), hinstance, std::ptr::null_mut());
    if RegisterClassExW(&wc) == 0 {
        return Err(format!("RegisterClassExW (fenêtre tray) a échoué (erreur {})", crate::win32::last_error()));
    }

    state.notify = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: load_icon(icon_path),
        ..Default::default()
    };
    copy_into_wide(tooltip, &mut state.notify.szTip);

    let hwnd = CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
        class_name.as_ptr(),
        std::ptr::null(),
        WS_OVERLAPPED,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        hinstance,
        Box::into_raw(state) as _,
    );
    if hwnd.is_null() {
        return Err(format!("CreateWindowExW (fenêtre tray) a échoué (erreur {})", crate::win32::last_error()));
    }
    ChangeWindowMessageFilterEx(hwnd, WM_TRAYICON, MSGFLT_ALLOW, std::ptr::null_mut());

    // GWLP_USERDATA est déjà posé (WM_NCCREATE, synchrone pendant
    // CreateWindowExW ci-dessus) -- state.hWnd/le hotkey ont besoin de ce
    // hwnd, qui n'existait pas encore quand `state` a été construit par
    // l'appelant.
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState;
    let state = &mut *ptr;
    state.notify.hWnd = hwnd;
    add_icon(&mut state.notify);
    if let Ok(h) = GlobalHotkey::register(hwnd, &state.hotkey_spec) {
        state.hotkey = Some(h);
        state.hotkey_enabled = true;
    }

    Ok(hwnd)
}
