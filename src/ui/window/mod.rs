//! La fenêtre popup principale : géométrie, contrôle EDIT natif pour la
//! recherche, rendu GDI de la liste de résultats, entrée clavier, tous les
//! modes (Window Switcher, Timer+rebond, Sticky Notes, Auto-restart,
//! sélecteur de thème).
//!
//! API publique de ce module : `create`/`show`/`hide`/`toggle`,
//! `menu_style`, les bascules Auto-restart/Copy History, et les deux points
//! d'entrée clavier appelés par la boucle de messages de main.rs
//! (`handle_edit_keydown`/`handle_backspace_on_empty`). Tout le reste est
//! réparti par responsabilité entre les sous-modules ci-dessous.
//!
//! Tous les modes partagent la même primitive de liste : `mode_items`
//! (les libellés à filtrer/afficher) + `filtered` (les indices retenus,
//! triés) + `selected`/`first_visible` -- voir `items` pour le détail.

mod actions;
mod app_state;
mod geometry;
mod items;
mod mode;
mod render;
mod timer;
mod wndproc;

use std::path::PathBuf;

use crate::core::models::App;
use crate::win32::gdi32::{RedrawWindow, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW};
use crate::win32::user32::{
    AddClipboardFormatListener, CreateWindowExW, GetWindowLongPtrW, IsWindowVisible, LoadCursorW,
    RegisterClassExW, RemoveClipboardFormatListener, SetWindowLongPtrW, ShowWindow,
    ES_AUTOHSCROLL, GWLP_USERDATA, GWLP_WNDPROC, IDC_ARROW, SW_HIDE, WS_CHILD, WS_CLIPCHILDREN,
    WS_EX_COMPOSITED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use crate::win32::{to_wstring, HWND};

use self::app_state::{AppState, Mode};
use self::geometry::{apply_geometry, compute_geometry, search_control_rect, work_area_under_cursor};
use self::timer::{bring_to_foreground, stop_bounce, SimpleRng};
use self::wndproc::{edit_subclass_proc, wndproc, ORIGINAL_EDIT_WNDPROC};

use super::gdi::simple_wndclass;
use super::theme::{self, ThemeConfig};

const WINDOW_CLASS_NAME: &str = "MAGILauncherPopupClass";

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    ptr.as_mut()
}

/// Repaint synchrone de la fenêtre principale ET de ses contrôles enfants
/// en un appel -- à faire après tout `SetWindowPos` qui change la taille
/// (stop_bounce, show, set_window_size_percent, adjust_border). Un
/// `InvalidateRect` se contente de marquer la zone à repeindre, et son
/// `WM_PAINT` (priorité la plus basse de la file) peut être retardé
/// derrière les messages générés par le `SetWindowPos` lui-même. Avec
/// `WS_EX_COMPOSITED`, la DWM donne à la fenêtre sa propre surface
/// hors-écran, dont les pixels nouvellement exposés par un agrandissement
/// démarrent transparents jusqu'à notre `BitBlt` : si la DWM présente une
/// frame avant ce WM_PAINT différé, la zone agrandie apparaît transparente
/// une frame, d'autant plus visiblement que l'agrandissement est large.
/// `RDW_UPDATENOW` ferme la course en forçant le `WM_PAINT` ici même.
unsafe fn force_repaint_now(hwnd: HWND) {
    RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASE | RDW_ALLCHILDREN);
}

pub struct WindowHandles {
    pub main: HWND,
    pub edit: HWND,
}

/// Crée la fenêtre popup (cachée) et son contrôle de recherche. `apps` et
/// `theme` sont déplacés dans l'état de la fenêtre (GWLP_USERDATA) -- tout
/// leur cycle de vie est ensuite géré par la fenêtre elle-même (libéré au
/// WM_DESTROY).
pub fn create(
    apps: Vec<App>,
    mut theme_cfg: ThemeConfig,
    base_dir: PathBuf,
    auto_restart_enabled: bool,
    auto_kill_enabled: bool,
    copy_history_enabled: bool,
) -> Result<WindowHandles, String> {
    let class_name = to_wstring(WINDOW_CLASS_NAME);
    let window_name = to_wstring("MAGI Launcher");

    unsafe {
        let wc = simple_wndclass(
            class_name.as_ptr(),
            Some(wndproc),
            std::ptr::null_mut(),
            LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
        );
        // ERROR_CLASS_ALREADY_EXISTS toléré, comme dans popup_menu::show().
        // En usage réel create() n'est appelée qu'une fois par process ;
        // sous cargo test en revanche, plusieurs #[test] l'appellent sous le
        // même nom de classe et peuvent tourner en parallèle dans le même
        // process -- une classe déjà enregistrée n'est alors pas une panne.
        const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;
        if RegisterClassExW(&wc) == 0 && crate::win32::last_error() != ERROR_CLASS_ALREADY_EXISTS {
            return Err(format!("RegisterClassExW a échoué (erreur {})", crate::win32::last_error()));
        }

        let themes_path = base_dir.join("themes.json");
        let themes_json_present = theme::load(&themes_path, &mut theme_cfg);
        let state_path = base_dir.join("state.json");
        theme::apply_prefs(&mut theme_cfg, &crate::core::state::load(&state_path).ui);

        let work = work_area_under_cursor();
        let geometry = compute_geometry(work, &theme_cfg);
        let g = geometry.window;

        let hwnd = CreateWindowExW(
            // WS_EX_COMPOSITED : demande à la DWM de composer la fenêtre ET
            // son enfant (EDIT de recherche) dans un tampon hors écran
            // commun. Le double buffering manuel de draw_scene ne couvre
            // que ce que la fenêtre dessine elle-même, pas le cycle de
            // peinture indépendant d'un contrôle enfant natif.
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_COMPOSITED,
            class_name.as_ptr(),
            window_name.as_ptr(),
            // WS_CLIPCHILDREN : sans ce style, le rendu GDI de la fenêtre
            // (fond de bordure sur tout le client, voir draw_scene) peint
            // PAR-DESSUS le contrôle enfant au lieu d'être découpé autour.
            WS_POPUP | WS_CLIPCHILDREN,
            g.left,
            g.top,
            super::gdi::rect_w(&g),
            super::gdi::rect_h(&g),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return Err(format!("CreateWindowExW a échoué (erreur {})", crate::win32::last_error()));
        }

        let edit_rect = search_control_rect(&geometry);

        let edit_class = to_wstring("EDIT");
        let edit_hwnd = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL as u32,
            edit_rect.left,
            edit_rect.top,
            super::gdi::rect_w(&edit_rect),
            super::gdi::rect_h(&edit_rect),
            hwnd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if edit_hwnd.is_null() {
            return Err(format!("création du contrôle EDIT échouée (erreur {})", crate::win32::last_error()));
        }

        // Sous-classement souris (voir edit_subclass_proc).
        ORIGINAL_EDIT_WNDPROC
            .store(GetWindowLongPtrW(edit_hwnd, GWLP_WNDPROC), std::sync::atomic::Ordering::Relaxed);
        SetWindowLongPtrW(edit_hwnd, GWLP_WNDPROC, edit_subclass_proc as *const () as isize);

        let notes_path = base_dir.join("notes.json");
        let restart_path = base_dir.join("restart.json");
        let kill_path = base_dir.join("kill.json");
        let notes = crate::core::json_list::load_notes(&notes_path);
        let restart_targets = crate::core::json_list::load_restart_list(&restart_path);
        let kill_targets = crate::core::json_list::load_kill_list(&kill_path);
        // Un seul thread de sondage pour Auto-restart ET Auto-kill. Lancé
        // toujours ; chaque facette s'active via son flag (Auto-kill (ré)arme
        // alors son délai de grâce, voir set_kill_enabled). Une facette
        // désactivée ne fait rien -- le thread ne s'arrête qu'au WM_DESTROY.
        let mut process_supervisor =
            crate::core::supervisor::ProcessSupervisor::new(restart_targets.clone(), kill_targets.clone());
        process_supervisor.set_restart_enabled(auto_restart_enabled);
        process_supervisor.set_kill_enabled(auto_kill_enabled);
        process_supervisor.start();
        let emoji = crate::core::emoji::load(&base_dir.join("emoji-test.txt"));

        let mut state = Box::new(AppState {
            mode_items: apps.iter().map(|a| a.name.clone()).collect(),
            mode_items_cache: (Vec::new(), Vec::new()),
            filtered: (0..apps.len()).collect(),
            apps,
            windows: Vec::new(),
            recycle_bin_items: Vec::new(),
            recycle_bin_pending: None,
            eject_drives: Vec::new(),
            emoji,
            notes,
            notes_path,
            restart_targets,
            restart_path,
            kill_targets,
            kill_path,
            process_supervisor,
            auto_restart_enabled,
            auto_kill_enabled,
            copy_history: crate::core::clipboard_history::ClipboardHistory::new(),
            copy_history_enabled,
            suppress_next_clipboard_capture: false,
            mode: Mode::Normal,
            selected: 0,
            first_visible: 0,
            display: self::app_state::SearchDisplay::List,
            theme: theme_cfg,
            theme_picker_original: None,
            themes_json_present,
            base_dir,
            themes_path,
            state_path,
            on_hotkey_reload: None,
            timer_deadline: None,
            bouncing: false,
            bounce_pos: (0.0, 0.0),
            bounce_vel: (0.0, 0.0),
            bounce_pre_geometry: None,
            bounce_pre_theme: None,
            rng: SimpleRng::new(),
            edit_hwnd,
            geometry,
            font_row: std::ptr::null_mut(),
            font_search: std::ptr::null_mut(),
            applied_font_family: String::new(),
            applied_font_px: 0,
            search_brush: std::ptr::null_mut(),
            list_bg_brush: std::ptr::null_mut(),
            selected_bg_brush: std::ptr::null_mut(),
            border_brush: std::ptr::null_mut(),
            mem_dc: std::ptr::null_mut(),
            mem_bitmap: std::ptr::null_mut(),
            mem_buffer_size: (0, 0),
            placeholder_wide: Vec::new(),
            text_margin: 0,
            recycle_bin_cache: std::cell::Cell::new(None),
        });
        self::app_state::apply_theme_visuals(&mut state);
        // Armé même si `show_clock` est faux au démarrage : un Reload peut
        // l'activer, et le tick reste un no-op d'ici là (voir
        // CLOCK_TIMER_ID).
        crate::win32::user32::SetTimer(hwnd, self::timer::CLOCK_TIMER_ID, 1000, None);

        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        if copy_history_enabled {
            AddClipboardFormatListener(hwnd);
        }

        Ok(WindowHandles { main: hwnd, edit: edit_hwnd })
    }
}

pub unsafe fn show(hwnd: HWND) {
    let Some(state) = get_state(hwnd) else { return };
    if state.bouncing {
        stop_bounce(hwnd, state);
    }
    apply_geometry(hwnd, state);
    self::mode::enter_mode(hwnd, state, Mode::Normal);
    bring_to_foreground(hwnd, state);
    force_repaint_now(hwnd);
}

pub unsafe fn hide(hwnd: HWND) {
    ShowWindow(hwnd, SW_HIDE);
}

/// Instantané du style courant (couleurs, police, épaisseur de bordure) --
/// utilisé par main.rs pour habiller le menu contextuel du tray
/// (ui::popup_menu) aux couleurs du thème actif, sans lui exposer
/// `AppState` en entier.
pub unsafe fn menu_style(hwnd: HWND) -> Option<(super::popup_menu::MenuColors, String, i32)> {
    let state = get_state(hwnd)?;
    let t = &state.theme.current;
    Some((
        super::popup_menu::MenuColors {
            list_background: t.list_background,
            list_text: t.list_text,
            selected_background: t.selected_background,
            selected_text: t.selected_text,
            border: t.border,
        },
        theme::resolve_font_family(&state.theme),
        state.theme.border_width,
    ))
}

/// Pose le pont vers le hotkey global (voir `AppState::on_hotkey_reload`) --
/// appelée une seule fois par main.rs juste après la création de la fenêtre
/// tray, puisque c'est seulement à ce moment-là que son hwnd existe.
pub unsafe fn set_hotkey_reload_hook(hwnd: HWND, hook: Box<dyn FnMut(&str, bool)>) {
    if let Some(state) = get_state(hwnd) {
        state.on_hotkey_reload = Some(hook);
    }
}

pub unsafe fn toggle(hwnd: HWND) {
    if IsWindowVisible(hwnd) != 0 {
        // Le raccourci global est une des trois sorties du rebond DVD (avec
        // Échap et le clic). Sans ce test, hide() masquerait la fenêtre en
        // laissant `bouncing` à true et BOUNCE_TIMER_ID armé, à
        // repositionner une fenêtre invisible jusqu'au prochain show().
        if let Some(state) = get_state(hwnd) {
            if state.bouncing {
                stop_bounce(hwnd, state);
            }
        }
        hide(hwnd);
    } else {
        show(hwnd);
    }
}

pub unsafe fn is_auto_restart_enabled(hwnd: HWND) -> bool {
    get_state(hwnd).is_none_or(|state| state.auto_restart_enabled)
}

pub unsafe fn is_auto_kill_enabled(hwnd: HWND) -> bool {
    get_state(hwnd).is_some_and(|state| state.auto_kill_enabled)
}

/// Bascule la facette Auto-restart du superviseur (menu du tray) : le thread
/// de sondage tourne en permanence, seul le flag change -- la liste
/// `restart_targets` reste intacte pendant la désactivation.
pub unsafe fn toggle_auto_restart(hwnd: HWND) {
    if let Some(state) = get_state(hwnd) {
        state.auto_restart_enabled = !state.auto_restart_enabled;
        state.process_supervisor.set_restart_enabled(state.auto_restart_enabled);
        // Persisté aussitôt (même principe que window_size/border) pour que
        // la bascule survive à un redémarrage. Best-effort.
        let _ = crate::core::state::commit_auto_restart_enabled(&state.state_path, state.auto_restart_enabled);
    }
}

/// Bascule la facette Auto-kill. Activer (ré)arme le délai de grâce de 10 s
/// (voir ProcessSupervisor::set_kill_enabled) : le premier kill n'aura donc
/// jamais lieu avant d'avoir eu le temps de rouvrir ce menu.
pub unsafe fn toggle_auto_kill(hwnd: HWND) {
    if let Some(state) = get_state(hwnd) {
        state.auto_kill_enabled = !state.auto_kill_enabled;
        state.process_supervisor.set_kill_enabled(state.auto_kill_enabled);
        let _ = crate::core::state::commit_auto_kill_enabled(&state.state_path, state.auto_kill_enabled);
    }
}

pub unsafe fn is_copy_history_enabled(hwnd: HWND) -> bool {
    get_state(hwnd).is_some_and(|state| state.copy_history_enabled)
}

/// Bascule l'historique de presse-papier (menu du tray) -- même persistance
/// immédiate que toggle_auto_restart. Enregistre/retire le listener au même
/// moment : désactiver coupe la capture (plus aucun WM_CLIPBOARDUPDATE
/// reçu) mais ne vide jamais l'historique déjà accumulé -- seule une
/// Suppr/Maj+Suppr dans Mode::CopyHistory le fait.
pub unsafe fn toggle_copy_history(hwnd: HWND) {
    if let Some(state) = get_state(hwnd) {
        state.copy_history_enabled = !state.copy_history_enabled;
        if state.copy_history_enabled {
            AddClipboardFormatListener(hwnd);
        } else {
            RemoveClipboardFormatListener(hwnd);
        }
        let _ = crate::core::state::commit_copy_history_enabled(&state.state_path, state.copy_history_enabled);
    }
}

pub(crate) use self::wndproc::{handle_backspace_on_empty, handle_edit_keydown};

/// Stress test et profilage mémoire de la fenêtre, sur un catalogue et un
/// thème synthétiques dans un dossier temporaire (jamais les vrais
/// apps.json/notes.json/restart.json). La fenêtre est pilotée par appel
/// direct de ses fonctions internes -- jamais SendInput/PostMessage, aucune
/// simulation d'événement OS -- et n'est jamais affichée (pas de show()).
///
/// SÉCURITÉ -- ce test ne doit JAMAIS :
/// - vider la vraie Corbeille (aucun sentinel `magi:*` dans le catalogue
///   factice, donc aucun chemin de code ne peut atteindre
///   core::recycle_bin::empty_async)
/// - activer/fermer/tuer une vraie fenêtre du bureau (Mode::Window
///   n'est jamais exercé au-delà de enter_mode/filtrage/move_selection --
///   jamais launch_selected ni on_delete pendant qu'on y est)
/// - lancer un vrai programme (les chemins du catalogue factice n'existent
///   pas sur le disque -- ShellExecuteExW échoue en silence, voir
///   core::launch)
#[cfg(test)]
mod stress_test {
    use super::actions::{launch_selected, on_delete, reload_config};
    use super::geometry::{adjust_border, set_window_size_percent};
    use super::items::{current_list_len, move_selection, refresh_filter};
    use super::mode::{enter_mode, exit_picker};
    use super::timer::{arm_timer, cancel_timer, stop_bounce};
    use super::wndproc::{handle_edit_keydown, wndproc};
    use super::*;
    use crate::win32::gdi32::{GetDC, ReleaseDC};
    use crate::win32::user32::{WA_INACTIVE, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_TAB, VK_UP};
    use crate::win32::WPARAM;
    use std::time::{Duration, Instant};

    fn sandbox_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_launcher_stress_{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fake_apps(n: usize) -> Vec<App> {
        let mut apps: Vec<App> = (0..n)
            .map(|i| App::new(format!("Stress App {i}"), format!("C:\\StressTest\\App{i}.exe"), None, false))
            .collect();
        // Chemin au-delà de MAX_PATH : exerce la troncature du rendu
        // (DrawTextW + DT_END_ELLIPSIS).
        let huge_path = format!("C:\\{}\\App.exe", "StressSubfolder".repeat(60));
        apps.push(App::new("Stress App With A Very Long Path Indeed".to_string(), huge_path, None, false));
        apps
    }

    fn process_memory_kb() -> usize {
        unsafe {
            let mut counters = crate::win32::kernel32::PROCESS_MEMORY_COUNTERS::default();
            counters.cb = std::mem::size_of::<crate::win32::kernel32::PROCESS_MEMORY_COUNTERS>() as u32;
            crate::win32::kernel32::K32GetProcessMemoryInfo(
                crate::win32::kernel32::GetCurrentProcess(),
                &mut counters,
                counters.cb,
            );
            counters.WorkingSetSize / 1024
        }
    }

    fn gdi_object_count() -> u32 {
        unsafe {
            crate::win32::user32::GetGuiResources(
                crate::win32::kernel32::GetCurrentProcess(),
                crate::win32::user32::GR_GDIOBJECTS,
            )
        }
    }

    fn user_object_count() -> u32 {
        unsafe {
            crate::win32::user32::GetGuiResources(
                crate::win32::kernel32::GetCurrentProcess(),
                crate::win32::user32::GR_USEROBJECTS,
            )
        }
    }

    unsafe fn pump_messages(hwnd: HWND, ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(ms);
        let mut msg = crate::win32::MSG::default();
        while Instant::now() < deadline {
            while crate::win32::user32::PeekMessageW(&mut msg, hwnd, 0, 0, crate::win32::user32::PM_REMOVE) != 0 {
                crate::win32::user32::TranslateMessage(&msg);
                crate::win32::user32::DispatchMessageW(&msg);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn stress_gui_complet() {
        let sandbox = sandbox_dir("gui");

        // Copie le vrai themes.json (100+ thèmes) pour exercer le sélecteur
        // sur un catalogue réel plutôt que sur l'unique thème de repli.
        let real_themes = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes.json");
        if real_themes.exists() {
            let _ = std::fs::copy(&real_themes, sandbox.join("themes.json"));
        }

        let apps = fake_apps(8);
        let handles =
            create(apps, ThemeConfig::default(), sandbox.clone(), true, false, false).expect("création fenêtre échouée");
        let hwnd = handles.main;

        // Requêtes volontairement adverses : division par zéro, expression
        // mal formée, hex court/trop long/majuscules, exposant énorme,
        // parenthèses profondément imbriquées, chaîne géante, unicode/
        // emoji, retours à la ligne/tabulations, aucune correspondance.
        let long_number = "9".repeat(400);
        let deep_parens = format!("{}1{}", "(".repeat(300), ")".repeat(300));
        let huge_query = "x".repeat(20_000);
        let queries: Vec<String> = vec![
            "stress".into(),
            "app".into(),
            "1".into(),
            "zzz-no-match".into(),
            "3+4*2".into(),
            "#3498db".into(),
            "".into(),
            "1/0".into(),
            "0/0".into(),
            "2+".into(),
            "#fff".into(),
            "#ABCDEF".into(),
            "#".into(),
            "#00000000000".into(),
            long_number,
            deep_parens,
            huge_query,
            "\u{1F600}\u{1F4A9} émoji tést \u{0000}".into(),
            "line1\nline2\ttab".into(),
            "999999999999999999999999999999*999999999999999999999999999999".into(),
        ];

        let iterations = 400;
        let mut baseline_mem = 0usize;
        let mut baseline_gdi = 0u32;

        for i in 0..iterations {
            let query = queries[i % queries.len()].clone();
            unsafe {
                let state = get_state(hwnd).expect("state manquant");
                super::app_state::set_edit_text(state.edit_hwnd, &query);
                refresh_filter(state);
            }

            unsafe {
                match i % 7 {
                    0 => {
                        // Thème : parcourt tout le catalogue réel avec
                        // preview live à chaque déplacement.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Theme);
                        let len = current_list_len(get_state(hwnd).unwrap());
                        for _ in 0..len.min(50) {
                            move_selection(get_state(hwnd).unwrap(), 1);
                        }
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    1 => {
                        // Window Switcher : LECTURE SEULE (jamais
                        // launch_selected/on_delete ici -- activerait/
                        // fermerait une vraie fenêtre du bureau).
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Window);
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, "a");
                        refresh_filter(get_state(hwnd).unwrap());
                        move_selection(get_state(hwnd).unwrap(), 3);
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, "");
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    2 => {
                        // Timer -- armé pour de vrai (rebond DVD réel) de
                        // temps en temps seulement, fenêtre jamais montrée.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Timer);
                        if i % 150 == 2 {
                            arm_timer(hwnd, get_state(hwnd).unwrap(), 1);
                            pump_messages(hwnd, 1300);
                            let state = get_state(hwnd).unwrap();
                            if state.bouncing {
                                stop_bounce(hwnd, state);
                            } else {
                                cancel_timer(hwnd, state);
                            }
                        } else {
                            cancel_timer(hwnd, get_state(hwnd).unwrap());
                        }
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    3 => {
                        // Notes : ajout PUIS suppression réels (persistance
                        // sur notes.json dans le sandbox), assertions
                        // dédiées.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Notes);
                        let note_text = format!("stress-note-{i}");
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &note_text);
                        refresh_filter(get_state(hwnd).unwrap());
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        assert!(
                            get_state(hwnd).unwrap().notes.contains(&note_text),
                            "note '{note_text}' jamais ajoutée"
                        );
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &note_text);
                        refresh_filter(get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        assert!(
                            !get_state(hwnd).unwrap().notes.contains(&note_text),
                            "note '{note_text}' jamais supprimée"
                        );
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    4 => {
                        // Auto-restart : même principe que Notes.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Restart);
                        let target = format!("C:\\StressTest\\AutoRestart{i}.exe");
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &target);
                        refresh_filter(get_state(hwnd).unwrap());
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        assert!(
                            get_state(hwnd).unwrap().restart_targets.contains(&target),
                            "cible '{target}' jamais ajoutée"
                        );
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &target);
                        refresh_filter(get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        assert!(
                            !get_state(hwnd).unwrap().restart_targets.contains(&target),
                            "cible '{target}' jamais supprimée"
                        );
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    5 => {
                        // Corbeille : lecture seule -- Entrée/Suppr y sont
                        // des no-ops voulus, donc sans risque. Le scan
                        // tourne sur un thread dédié : on laisse le temps au
                        // timer de sondage de boucler au moins une fois,
                        // plutôt que de ne tester que la liste vide.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::RecycleBin);
                        pump_messages(hwnd, 150);
                        move_selection(get_state(hwnd).unwrap(), 2);
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    _ => {
                        // Copy History : ajout puis suppression réels (RAM
                        // seulement), même esprit que Notes. push() est
                        // appelé directement, jamais via le presse-papier
                        // système : aucun événement OS n'est simulé ici.
                        let state = get_state(hwnd).unwrap();
                        let entry_text = format!("stress-copy-{i}");
                        state.copy_history.push(entry_text.clone());
                        enter_mode(hwnd, state, Mode::CopyHistory);
                        assert!(
                            get_state(hwnd).unwrap().mode_items.iter().any(|s| s == &entry_text),
                            "entrée copy-history '{entry_text}' absente de mode_items"
                        );
                        super::app_state::set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &entry_text);
                        refresh_filter(get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        assert!(
                            !get_state(hwnd).unwrap().mode_items.iter().any(|s| s == &entry_text),
                            "entrée copy-history '{entry_text}' jamais supprimée"
                        );
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                }
            }

            // Redimensionnement en direct (Ctrl+1..9/0 et Ctrl+-/+ en usage
            // réel), appelé directement et non via handle_edit_keydown :
            // GetKeyState y lit l'état du clavier physique, non simulable
            // sans SendInput (proscrit ici). Peu fréquent -- chaque appel
            // écrit sur disque -- et cyclé sur quelques tailles.
            if i % 40 == 0 {
                unsafe {
                    let state = get_state(hwnd).unwrap();
                    set_window_size_percent(hwnd, state, [10, 50, 90][i / 40 % 3]);
                    adjust_border(hwnd, get_state(hwnd).unwrap(), if i % 80 == 0 { 1 } else { -1 });
                }
            }

            // Navigation clavier universelle via le VRAI point d'entrée
            // (handle_edit_keydown), pas un appel direct à enter_mode --
            // exerce la normalisation Ctrl+S/W/D/A et les règles Tab/Échap
            // telles qu'un vrai WM_KEYDOWN les déclencherait.
            unsafe {
                handle_edit_keydown(hwnd, VK_DOWN);
                handle_edit_keydown(hwnd, VK_UP);
                handle_edit_keydown(hwnd, VK_TAB); // -> Window Switcher
                handle_edit_keydown(hwnd, VK_ESCAPE); // -> retour Normal
            }

            // Perte de focus pendant une preview de thème non validée --
            // exerce la branche WM_ACTIVATE/WA_INACTIVE.
            unsafe {
                if i % 15 == 0 {
                    let state = get_state(hwnd).unwrap();
                    enter_mode(hwnd, state, Mode::Theme);
                    move_selection(get_state(hwnd).unwrap(), 1);
                    wndproc(hwnd, crate::win32::user32::WM_ACTIVATE, WA_INACTIVE as WPARAM, 0);
                }

                // Lancement "normal" d'une appli factice -- chemin
                // inexistant, ShellExecuteExW échoue en silence
                // (SEE_MASK_FLAG_NO_UI), aucun programme ne démarre
                // réellement.
                if i % 7 == 0 {
                    let state = get_state(hwnd).unwrap();
                    enter_mode(hwnd, state, Mode::Normal);
                    super::app_state::set_edit_text(state.edit_hwnd, &format!("Stress App {}", i % 8));
                    refresh_filter(get_state(hwnd).unwrap());
                    launch_selected(hwnd, get_state(hwnd).unwrap());
                }

                // Reload en alternant show_clock (recharge aussi le vrai
                // themes.json copié dans le sandbox).
                if i % 25 == 0 {
                    reload_config(hwnd, get_state(hwnd).unwrap());
                }
            }

            if i == 50 {
                baseline_mem = process_memory_kb();
                baseline_gdi = gdi_object_count();
            }
            if i > 50 && i % 100 == 0 {
                eprintln!(
                    "iter {i}: mem={}KB (baseline {baseline_mem}KB) gdi={} user={}",
                    process_memory_kb(),
                    gdi_object_count(),
                    user_object_count()
                );
            }
        }

        let final_mem = process_memory_kb();
        let final_gdi = gdi_object_count();
        let final_user = user_object_count();
        eprintln!(
            "=== final : mem={final_mem}KB (baseline {baseline_mem}KB)  gdi={final_gdi} (baseline {baseline_gdi})  user={final_user} ==="
        );

        unsafe {
            crate::win32::user32::DestroyWindow(hwnd);
        }
        let _ = std::fs::remove_dir_all(&sandbox);

        // Une croissance linéaire du nombre d'objets GDI/USER avec les
        // itérations trahirait une fuite de handle (police/pinceau jamais
        // détruits) -- marge x3 pour tolérer la variance normale sans être
        // aveugle à une vraie fuite.
        assert!(
            (final_gdi as u64) <= (baseline_gdi.max(20) as u64) * 3,
            "fuite d'objets GDI suspectée : {baseline_gdi} -> {final_gdi}"
        );
    }

    /// Régression sur l'invariant : aucun handle GDI n'est NULL après une
    /// transition de taille 30% -> 100% -> 30%, ni après les actualisations
    /// qui suivent (défilement, tick de minuteur, Suppr, Tab) -- protège la
    /// recréation des handles GDI/USER (police, pinceaux, mem_dc/
    /// mem_bitmap) à chaque changement de taille de fenêtre (voir
    /// ensure_scene_buffer/apply_theme_visuals).
    #[test]
    fn transition_30_100_30_ne_laisse_jamais_un_handle_gdi_nul() {
        let sandbox = sandbox_dir("gdi_30_100_30");
        let apps = fake_apps(8);
        let handles =
            create(apps, ThemeConfig::default(), sandbox.clone(), true, false, false).expect("création fenêtre échouée");
        let hwnd = handles.main;

        // ensure_scene_buffer appelée directement (ce que fait WM_PAINT)
        // plutôt qu'en attendant un vrai WM_PAINT : cette fenêtre de test
        // n'est jamais montrée, et Windows ne dispatche pas WM_PAINT de
        // façon fiable pour une fenêtre jamais affichée, même avec
        // RDW_UPDATENOW. GetDC(NULL) ne sert que de référence de format de
        // pixels à CreateCompatibleDC/CreateCompatibleBitmap.
        unsafe fn assert_handles_valides(hwnd: HWND, label: &str) {
            unsafe {
                let state = get_state(hwnd).expect("state manquant");
                let screen_dc = GetDC(std::ptr::null_mut());
                super::wndproc::ensure_scene_buffer(screen_dc, state);
                ReleaseDC(std::ptr::null_mut(), screen_dc);
            }
            unsafe {
                let state = get_state(hwnd).expect("state manquant");
                assert!(!state.font_row.is_null(), "{label}: font_row NULL");
                assert!(!state.font_search.is_null(), "{label}: font_search NULL");
                assert!(!state.search_brush.is_null(), "{label}: search_brush NULL");
                assert!(!state.list_bg_brush.is_null(), "{label}: list_bg_brush NULL");
                assert!(!state.selected_bg_brush.is_null(), "{label}: selected_bg_brush NULL");
                assert!(!state.border_brush.is_null(), "{label}: border_brush NULL");
                assert!(!state.mem_dc.is_null(), "{label}: mem_dc NULL");
                assert!(!state.mem_bitmap.is_null(), "{label}: mem_bitmap NULL");
                let expected =
                    (crate::ui::gdi::rect_w(&state.geometry.window).max(1), crate::ui::gdi::rect_h(&state.geometry.window).max(1));
                assert_eq!(state.mem_buffer_size, expected, "{label}: mem_buffer_size ne suit pas la géométrie");
            }
            eprintln!("{label}: gdi={} user={} mem={}KB", gdi_object_count(), user_object_count(), process_memory_kb());
        }

        unsafe {
            let state = get_state(hwnd).unwrap();
            set_window_size_percent(hwnd, state, 30);
            assert_handles_valides(hwnd, "apres 30%");

            let state = get_state(hwnd).unwrap();
            set_window_size_percent(hwnd, state, 100);
            assert_handles_valides(hwnd, "apres 100%");

            let state = get_state(hwnd).unwrap();
            set_window_size_percent(hwnd, state, 30);
            assert_handles_valides(hwnd, "apres retour a 30%");

            // Les actualisations à couvrir dans cet état : défilement
            // (flèches), tick de minuteur (InvalidateRect direct, comme
            // WM_TIMER(CLOCK_TIMER_ID)), Suppr, Tab.
            for _ in 0..20 {
                handle_edit_keydown(hwnd, VK_DOWN);
            }
            crate::win32::gdi32::InvalidateRect(hwnd, std::ptr::null(), 0); // tick d'horloge
            handle_edit_keydown(hwnd, VK_DELETE);
            handle_edit_keydown(hwnd, VK_TAB);
            handle_edit_keydown(hwnd, VK_ESCAPE);
            assert_handles_valides(hwnd, "apres scroll/timer/sup/tab");

            crate::win32::user32::DestroyWindow(hwnd);
        }
        let _ = std::fs::remove_dir_all(&sandbox);
    }

    #[test]
    fn rejette_des_reglages_de_theme_extremes_sans_planter() {
        let work = crate::win32::RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        let sandbox = sandbox_dir("theme_extreme");
        let cases = [
            r##"{"theme":"t","window_width_fraction":1e300,"border":1000000000,"themes":{"t":{"search_background":"#000000","search_text":"#ffffff","list_background":"#000000","list_text":"#ffffff","selected_background":"#000000","selected_text":"#ffffff","border":"#000000"}}}"##,
            r##"{"theme":"t","window_width_fraction":-1,"border":-2147483648,"themes":{"t":{"search_background":"#000000","search_text":"#ffffff","list_background":"#000000","list_text":"#ffffff","selected_background":"#000000","selected_text":"#ffffff","border":"#000000"}}}"##,
            r##"{"theme":"t","window_width_fraction":0,"border":2147483647,"themes":{"t":{"search_background":"#000000","search_text":"#ffffff","list_background":"#000000","list_text":"#ffffff","selected_background":"#000000","selected_text":"#ffffff","border":"#000000"}}}"##,
        ];
        for (i, case) in cases.iter().enumerate() {
            let path = sandbox.join(format!("themes_{i}.json"));
            std::fs::write(&path, case).unwrap();
            let mut cfg = ThemeConfig::default();
            theme::load(&path, &mut cfg);
            let g = compute_geometry(work, &cfg);
            eprintln!(
                "cas {i} -> fraction={} border={} -> window left={} top={} right={} bottom={}",
                cfg.window_width_fraction, cfg.border_width, g.window.left, g.window.top, g.window.right,
                g.window.bottom
            );
        }
        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
