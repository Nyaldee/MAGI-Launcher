// Sous-système GUI (pas console) -- sans ça, Windows ouvre une invite de
// commandes derrière l'appli à chaque lancement (comportement par défaut
// d'un `fn main()`, pensé pour des outils en ligne de commande).
#![windows_subsystem = "windows"]

mod core;
mod json;
mod ui;
mod win32;

use std::path::{Path, PathBuf};

use core::config::load_config;
use core::hotkey::GlobalHotkey;
use ui::popup_menu::{self, MenuItem};
use ui::theme::ThemeConfig;
use ui::tray::TrayState;
use ui::window;
use win32::kernel32::{CreateMutexW, ERROR_ALREADY_EXISTS};
use win32::user32::{
    DestroyWindow, GetMessageW, MessageBoxW, SetProcessDPIAware, SetProcessDpiAwareness, TranslateMessage,
    MB_ICONERROR, MB_OK, PROCESS_PER_MONITOR_DPI_AWARE, VK_BACK, WM_HOTKEY, WM_KEYDOWN,
};
use win32::{last_error, to_wstring, MSG};

const INSTANCE_MUTEX_NAME: &str = "MAGILauncherSingleInstance";
const GITHUB_URL: &str = "https://github.com/Nyaldee/MAGI-Launcher";

/// Remplace eprintln! pour les échecs de démarrage : sans console (sous-
/// système GUI ci-dessus), un message sur stderr ne serait jamais vu.
fn show_startup_error(message: &str) {
    let title = to_wstring("MAGI Launcher");
    let text = to_wstring(message);
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

/// Dossier de l'exécutable en cours -- apps.json/themes.json vivent
/// toujours à côté de lui (jamais empaquetés dans le binaire), pour que
/// l'utilisateur puisse les éditer à la main sans recompiler. Icon.ico n'en
/// fait PAS partie : l'icône du tray est embarquée comme ressource dans
/// l'exe lui-même (voir ui::tray::load_icon), Icon.ico à côté de l'exe
/// n'étant plus qu'un repli si jamais cette ressource manquait.
fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// True si on tient bien le verrou (aucune autre instance active). Un
/// mutex nommé plutôt qu'un fichier lock : Windows le libère toujours tout
/// seul à la sortie du process propriétaire, même tué de force -- pas
/// besoin d'une logique de récupération d'un verrou resté bloqué.
fn acquire_single_instance_lock() -> bool {
    let name = to_wstring(INSTANCE_MUTEX_NAME);
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        show_startup_error(&format!("CreateMutexW a échoué (erreur {})", last_error()));
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
    let items = vec![
        MenuItem::Entry("toggle_hotkey".to_string(), hotkey_label.to_string()),
        MenuItem::Entry("toggle_autorestart".to_string(), autorestart_label.to_string()),
        MenuItem::Entry("github".to_string(), "GitHub".to_string()),
        MenuItem::Entry("quit".to_string(), "Quit".to_string()),
    ];
    popup_menu::show(hwnd, colors, &font_family, border_width, items)
}

fn main() {
    // Vérifié avant DPI/tout le reste -- si une instance tourne déjà,
    // inutile de payer la suite pour quitter aussitôt. Silencieux,
    // volontairement : une instance déjà lancée n'est pas une erreur
    // (double-clic accidentel, raccourci de démarrage relancé...).
    if !acquire_single_instance_lock() {
        return;
    }

    enable_dpi_awareness();

    let bdir = base_dir();
    let (hotkey_spec, apps) = match load_config(&bdir.join("apps.json")) {
        Ok(v) => v,
        Err(e) => {
            show_startup_error(&format!("Impossible de charger apps.json : {}", e));
            return;
        }
    };

    let handles = match window::create(apps, ThemeConfig::default(), bdir.clone()) {
        Ok(h) => h,
        Err(e) => {
            show_startup_error(&e);
            return;
        }
    };

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
                }
                Some(id) if id == "toggle_autorestart" => {
                    window::toggle_auto_restart(state.main_hwnd);
                }
                Some(id) if id == "github" => {
                    let _ = core::launch::launch(GITHUB_URL, None, false);
                }
                Some(id) if id == "quit" => {
                    DestroyWindow(tray_hwnd);
                }
                _ => {}
            }
        }),
    ));

    let icon_path = bdir.join("Icon.ico");
    let tooltip = format!("MAGI Launcher ({})", hotkey_spec);
    let tray_hwnd = match unsafe { ui::tray::create_tray_window(state, &icon_path, &tooltip) } {
        Ok(h) => h,
        Err(e) => {
            show_startup_error(&e);
            return;
        }
    };

    // Boucle de messages unique sur le thread principal. Les WM_KEYDOWN
    // destinés au contrôle EDIT de recherche sont interceptés ICI, avant
    // Translate/Dispatch, pour les touches de navigation (flèches, Entrée,
    // Échap...) plutôt que de sous-classer le contrôle -- moins de code
    // pour un besoin aussi ciblé (un seul contrôle enfant).
    // Le clic sur le tray est traité DANS tray_wndproc (via DispatchMessageW
    // normal), pas court-circuité ici -- voir ui::tray.
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.hwnd == handles.edit && msg.message == WM_KEYDOWN {
                // Retour arrière sur une recherche déjà vide : sort du
                // picker actif plutôt que de ne rien faire (vérifié avant
                // Translate/Dispatch, donc avant que l'EDIT n'ait la main).
                if msg.wParam as u16 == VK_BACK as u16 && window::handle_backspace_on_empty(handles.main) {
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
