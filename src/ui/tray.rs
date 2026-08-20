//! Icône dans la zone de notification (Shell_NotifyIconW), sans aucune
//! dépendance tierce. Le menu contextuel n'est PAS géré ici -- voir
//! ui::popup_menu.
//!
//! L'icône est portée par une fenêtre dédiée, invisible, dont la
//! configuration est contrainte : une vraie fenêtre WS_OVERLAPPED (ni
//! WS_POPUP ni HWND_MESSAGE) avec les styles étendus posés plus bas, et
//! surtout un WndProc qui traite le clic lui-même. Shell_NotifyIconW peut
//! réussir tout en ne délivrant jamais WM_TRAYICON si son message de rappel
//! est court-circuité dans la boucle de messages : il doit passer par
//! DispatchMessageW jusqu'au WndProc. Même montage que le crate `tray-icon`
//! de tauri-apps.

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

/// Reçoit le hwnd de la fenêtre TRAY (pas main_hwnd) : le hotkey global est
/// enregistré sur elle, et la bascule du menu doit pouvoir le réenregistrer.
type OnRightClick = Box<dyn FnMut(HWND, &mut TrayState)>;

pub struct TrayState {
    pub main_hwnd: HWND,
    pub hotkey_spec: String,
    pub hotkey: Option<GlobalHotkey>,
    pub hotkey_enabled: bool,
    notify: NOTIFYICONDATAW,
    /// Appelé sur clic droit pour construire et afficher le menu thémé.
    /// Câblé depuis main.rs : ce module reste ainsi indépendant de
    /// ui::window et ui::popup_menu.
    pub on_right_click: OnRightClick,
}

impl TrayState {
    /// `notify` est laissé vide ici : il exige le hwnd de la fenêtre tray,
    /// qui n'existe pas encore, et sera renseigné par `create_tray_window`.
    pub fn new(main_hwnd: HWND, hotkey_spec: String, on_right_click: OnRightClick) -> TrayState {
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
            Shell_NotifyIconW(NIM_MODIFY, &self.notify);
        }
    }
}

unsafe extern "system" fn tray_wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Le pointeur d'état arrive via lpCreateParams dès WM_NCCREATE, donc
    // avant le retour de CreateWindowExW : stocké dans GWLP_USERDATA ici,
    // pas après coup, pour qu'aucun message précoce ne le trouve absent.
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
                // Sortie temporaire de la closure : elle reçoit `&mut state`,
                // qu'elle ne pourrait pas emprunter en restant dans state.
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

/// Charge l'icône embarquée dans l'exe (voir build.rs/winresource, ressource
/// ID 1) : fiable même si l'exe est copié seul, sans Icon.ico à côté.
/// `icon_path` sert de repli au cas où la ressource embarquée manquerait.
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
    // Icône générique de Windows en dernier recours : une NOTIFYICONDATAW
    // sans hIcon valide ne s'affiche pas du tout dans la zone de
    // notification.
    unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) }
}

unsafe fn add_icon(data: &mut NOTIFYICONDATAW) {
    Shell_NotifyIconW(NIM_ADD, data);
}

unsafe fn remove_icon(data: &mut NOTIFYICONDATAW) {
    Shell_NotifyIconW(NIM_DELETE, data);
    DestroyIcon(data.hIcon);
}

/// Crée la fenêtre invisible dédiée (voir l'en-tête de fichier), y ajoute
/// l'icône et enregistre le hotkey global. `main_hwnd`/`hotkey_spec`/
/// `on_right_click` sont déjà posés dans `state` ; l'icône et le hotkey le
/// sont ici, une fois le hwnd connu. `start_enabled` reflète
/// "hotkey_enabled" persisté dans apps.json (voir core::config) : le hotkey
/// n'est réenregistré au démarrage que s'il était actif à la dernière
/// bascule tray.
pub unsafe fn create_tray_window(
    mut state: Box<TrayState>,
    icon_path: &Path,
    tooltip: &str,
    start_enabled: bool,
) -> Result<HWND, String> {
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

    // GWLP_USERDATA est déjà posé : WM_NCCREATE est délivré de façon
    // synchrone pendant le CreateWindowExW ci-dessus. C'est le premier
    // moment où notify.hWnd et le hotkey peuvent recevoir ce hwnd.
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState;
    let state = &mut *ptr;
    state.notify.hWnd = hwnd;
    add_icon(&mut state.notify);
    if start_enabled {
        if let Ok(h) = GlobalHotkey::register(hwnd, &state.hotkey_spec) {
            state.hotkey = Some(h);
            state.hotkey_enabled = true;
        }
    }

    Ok(hwnd)
}
