// Sous-système GUI : sans ça, Windows ouvre une invite de commandes
// derrière l'appli à chaque lancement.
#![windows_subsystem = "windows"]

mod core;
mod json;
mod ui;
mod win32;

use std::path::{Path, PathBuf};

use core::config::load_all;
use core::hotkey::GlobalHotkey;
use ui::popup_menu::{self, MenuItem};
use ui::theme::ThemeConfig;
use ui::tray::TrayState;
use ui::window;
use win32::kernel32::{CreateMutexW, ERROR_ALREADY_EXISTS};
use win32::user32::{
    DestroyWindow, GetMessageW, SetProcessDPIAware, SetProcessDpiAwareness, TranslateMessage,
    PROCESS_PER_MONITOR_DPI_AWARE, VK_BACK, WM_HOTKEY, WM_KEYDOWN,
};
use win32::{last_error, to_wstring, MSG};

const INSTANCE_MUTEX_NAME: &str = "MAGILauncherSingleInstance";
const GITHUB_URL: &str = "https://github.com/Nyaldee/MAGI-Launcher";

/// Dossier de l'exécutable : apps.json/themes.json vivent à côté de lui,
/// jamais empaquetés dans le binaire, pour rester éditables à la main.
/// Icon.ico fait exception -- l'icône du tray est embarquée comme ressource
/// de l'exe (voir ui::tray::load_icon), le fichier voisin n'est qu'un repli.
fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `true` si le verrou est tenu (aucune autre instance active). Mutex nommé
/// plutôt que fichier lock : Windows le libère à la sortie du process
/// propriétaire, même tué de force -- aucune récupération de verrou bloqué à
/// écrire. Un `CreateMutexW` en échec compte comme "verrou non tenu".
fn acquire_single_instance_lock() -> bool {
    let name = to_wstring(INSTANCE_MUTEX_NAME);
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return false;
    }
    last_error() != ERROR_ALREADY_EXISTS
}

/// Sans ça, Windows applique un redimensionnement bitmap flou à toute la
/// fenêtre sur un écran mis à l'échelle (125%/150%...). Repli sur l'API
/// user32 historique si l'API shcore (Windows 8.1+) est indisponible.
fn enable_dpi_awareness() {
    unsafe {
        if SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE) != 0 {
            SetProcessDPIAware();
        }
    }
}

unsafe fn show_tray_menu(hwnd: win32::HWND, hotkey_enabled: bool) -> Option<String> {
    let (colors, font_family, border_width) = window::menu_style(hwnd)?;
    let hotkey_label = if hotkey_enabled { "Disable Hotkey" } else { "Enable Hotkey" };
    let autorestart_label =
        if window::is_auto_restart_enabled(hwnd) { "Disable Auto-restart" } else { "Enable Auto-restart" };
    let copy_history_label =
        if window::is_copy_history_enabled(hwnd) { "Disable Copy History" } else { "Enable Copy History" };
    let items = vec![
        MenuItem::Entry("toggle_hotkey".to_string(), hotkey_label.to_string()),
        MenuItem::Entry("toggle_autorestart".to_string(), autorestart_label.to_string()),
        MenuItem::Entry("toggle_copy_history".to_string(), copy_history_label.to_string()),
        MenuItem::Entry("github".to_string(), "GitHub".to_string()),
        MenuItem::Entry("quit".to_string(), "Quit".to_string()),
    ];
    popup_menu::show(hwnd, colors, &font_family, border_width, items)
}

fn main() {
    // Avant DPI et tout le reste : inutile de payer l'initialisation pour
    // quitter aussitôt. Sortie silencieuse -- une instance déjà lancée n'est
    // pas une erreur (double-clic accidentel, raccourci de démarrage relancé).
    if !acquire_single_instance_lock() {
        return;
    }

    enable_dpi_awareness();

    // apps.json absent/invalide, création de fenêtre ou de tray échouée : le
    // lanceur ne démarre simplement pas. Aucune popup d'erreur, à aucun stade.
    let bdir = base_dir();
    let Ok(cfg) = load_all(&bdir) else { return };
    let hotkey_spec = cfg.hotkey;

    let Ok(handles) = window::create(
        cfg.apps,
        ThemeConfig::default(),
        bdir.clone(),
        cfg.auto_restart_enabled,
        cfg.copy_history_enabled,
    ) else {
        return;
    };

    let commit_path = bdir.join("apps.json");
    let state = Box::new(TrayState::new(
        handles.main,
        hotkey_spec.clone(),
        Box::new(move |tray_hwnd, state| unsafe {
            match show_tray_menu(state.main_hwnd, state.hotkey_enabled) {
                Some(id) if id == "toggle_hotkey" => {
                    if state.hotkey_enabled {
                        state.hotkey = None; // Drop -> désenregistre (voir GlobalHotkey)
                        state.hotkey_enabled = false;
                        state.set_tooltip("MAGI Launcher (hotkey disabled)");
                    } else if let Ok(h) = GlobalHotkey::register(tray_hwnd, &state.hotkey_spec) {
                        state.hotkey = Some(h);
                        state.hotkey_enabled = true;
                        state.set_tooltip(&format!("MAGI Launcher ({})", state.hotkey_spec));
                    }
                    // Persisté aussitôt (comme auto_restart_enabled/
                    // copy_history_enabled dans ui::window) pour que la
                    // bascule survive à un redémarrage. Best-effort.
                    let _ = core::config::commit_bool_setting(&commit_path, "hotkey_enabled", state.hotkey_enabled);
                }
                Some(id) if id == "toggle_autorestart" => {
                    window::toggle_auto_restart(state.main_hwnd);
                }
                Some(id) if id == "toggle_copy_history" => {
                    window::toggle_copy_history(state.main_hwnd);
                }
                Some(id) if id == "github" => {
                    let _ = core::launch::launch(GITHUB_URL, None, false);
                }
                Some(id) if id == "quit" => {
                    // Ordre imposé. DestroyWindow envoie WM_DESTROY de façon
                    // SYNCHRONE : c'est le seul moment où AppState (dont
                    // copy_history) est réellement droppé, donc où
                    // core::clipboard_history::LockedString efface sa mémoire
                    // à zéro (voir son Drop). Laisser le process quitter sans
                    // cet appel rendrait la mémoire à l'OS telle quelle, sans
                    // jamais exécuter cet effacement. tray_hwnd ensuite : son
                    // WM_DESTROY appelle PostQuitMessage, qui termine la
                    // boucle de messages de main().
                    DestroyWindow(state.main_hwnd);
                    DestroyWindow(tray_hwnd);
                }
                _ => {}
            }
        }),
    ));

    let icon_path = bdir.join("Icon.ico");
    let tooltip = if cfg.hotkey_enabled {
        format!("MAGI Launcher ({})", hotkey_spec)
    } else {
        "MAGI Launcher (hotkey disabled)".to_string()
    };
    let Ok(tray_hwnd) = (unsafe { ui::tray::create_tray_window(state, &icon_path, &tooltip, cfg.hotkey_enabled) })
    else {
        return;
    };

    // Boucle de messages unique sur le thread principal. Les WM_KEYDOWN
    // destinés au contrôle EDIT de recherche sont interceptés ICI, avant
    // Translate/Dispatch, pour capter les touches de navigation (flèches,
    // Entrée, Échap) avant que l'EDIT ne les consomme -- plus léger qu'un
    // sous-classement pour un seul contrôle enfant.
    // Le clic sur le tray, lui, est traité DANS tray_wndproc via
    // DispatchMessageW et n'est volontairement pas court-circuité ici (voir
    // ui::tray).
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.hwnd == handles.edit && msg.message == WM_KEYDOWN {
                // Retour arrière sur une recherche déjà vide : sort du picker
                // actif plutôt que de ne rien faire.
                if msg.wParam as u16 == VK_BACK && window::handle_backspace_on_empty(handles.main) {
                    continue;
                }
                if window::handle_edit_keydown(handles.main, msg.wParam as u16) {
                    continue;
                }
            }
            if msg.hwnd == tray_hwnd && msg.message == WM_HOTKEY {
                window::toggle(handles.main);
                continue;
            }
            TranslateMessage(&msg);
            win32::user32::DispatchMessageW(&msg);
        }
    }
}
