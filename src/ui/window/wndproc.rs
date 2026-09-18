//! La vraie procédure de fenêtre (peinture, souris, presse-papier, timers,
//! cycle de vie), le sous-classement souris de l'EDIT de recherche, et le
//! point d'entrée clavier appelé par la boucle de messages de main.rs.

use crate::win32::gdi32::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint, FillRect,
    InvalidateRect, SelectObject, SetBkColor, SetBkMode, SetTextColor, HDC, OPAQUE, PAINTSTRUCT, SRCCOPY,
};
use crate::win32::user32::{
    CallWindowProcW, DefWindowProcW, GetCaretPos, GetClientRect, GetKeyState, GetParent, GetWindowLongPtrW,
    GetWindowTextLengthW, HideCaret, KillTimer, LoadCursorW, SetCaretPos, SetCursor, SetFocus,
    SetWindowLongPtrW, ShowCaret, IDC_ARROW, RemoveClipboardFormatListener,
    VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN,
    VK_ESCAPE, VK_LEFT, VK_OEM_MINUS, VK_OEM_PLUS, VK_RETURN, VK_RIGHT, VK_S, VK_SHIFT, VK_TAB, VK_UP, VK_W,
    WA_INACTIVE, WM_ACTIVATE, WM_CLOSE, WM_CLIPBOARDUPDATE, WM_COMMAND, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SETCURSOR, WM_TIMER, EN_CHANGE, GWLP_USERDATA, WNDPROC,
};
use crate::win32::{get_clipboard_text, HWND, LPARAM, LRESULT, POINT, RECT, UINT, WPARAM};

use super::app_state::{AppState, Mode};
use super::geometry::{adjust_border, rows_band_rect, set_window_size_percent, VISIBLE_ROWS};
use super::get_state;
use super::hide;
use super::items::{rebuild_copy_history_items, refresh_filter};
use super::mode::{cancel_uncommitted_theme_preview, enter_mode, exit_picker};
use super::actions::{launch_selected, on_delete, reveal_or_edit};
use super::render::{draw_placeholder, draw_scene};
use super::timer::{bounce_tick, fire, stop_bounce, CLOCK_TIMER_ID, COUNTDOWN_TIMER_ID, BOUNCE_TIMER_ID, RECYCLE_BIN_POLL_TIMER_ID};
use super::items::poll_recycle_bin;

/// (Re)crée `state.mem_dc`/`state.mem_bitmap` pour qu'ils fassent
/// exactement la taille de la fenêtre -- ne fait rien si la taille n'a pas
/// changé depuis le dernier appel (même garde-fou que les polices/pinceaux,
/// voir mem_buffer_size). `hdc_screen` sert uniquement de référence de
/// format de pixels à `CreateCompatibleDC`/`CreateCompatibleBitmap`, jamais
/// dessiné dedans.
pub(crate) unsafe fn ensure_scene_buffer(hdc_screen: HDC, state: &mut AppState) {
    let w = crate::ui::gdi::rect_w(&state.geometry.window).max(1);
    let h = crate::ui::gdi::rect_h(&state.geometry.window).max(1);
    if state.mem_buffer_size == (w, h) && !state.mem_dc.is_null() {
        return;
    }
    if !state.mem_bitmap.is_null() {
        DeleteObject(state.mem_bitmap as _);
    }
    if !state.mem_dc.is_null() {
        DeleteDC(state.mem_dc);
    }
    state.mem_dc = CreateCompatibleDC(hdc_screen);
    state.mem_bitmap = CreateCompatibleBitmap(hdc_screen, w, h);
    SelectObject(state.mem_dc, state.mem_bitmap as _);
    state.mem_buffer_size = (w, h);
}

pub(crate) unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            // Le fond entier est déjà repeint dans draw_scene à chaque
            // WM_PAINT -- laisser l'effacement par défaut ferait clignoter
            // la fenêtre pour rien.
            1
        }
        WM_PAINT => {
            // Dessiné dans state.mem_dc puis présenté d'un seul BitBlt
            // atomique -- voir mem_dc dans AppState.
            if let Some(state) = get_state(hwnd) {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                ensure_scene_buffer(hdc, state);
                draw_scene(state.mem_dc, state, &ps.rcPaint);
                let w = ps.rcPaint.right - ps.rcPaint.left;
                let h = ps.rcPaint.bottom - ps.rcPaint.top;
                if w > 0 && h > 0 {
                    BitBlt(hdc, ps.rcPaint.left, ps.rcPaint.top, w, h, state.mem_dc, ps.rcPaint.left, ps.rcPaint.top, SRCCOPY);
                }
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            // L'EDIT de recherche doit se fondre dans la barre (même fond
            // que search_background). WM_CTLCOLORSTATIC ne concerne aucun
            // contrôle actuel -- l'horloge n'en est pas un, voir
            // draw_clock_text -- mais couvre un futur enfant STATIC.
            if let Some(state) = get_state(hwnd) {
                let hdc = wparam as HDC;
                SetTextColor(hdc, state.theme.current.search_text);
                SetBkColor(hdc, state.theme.current.search_background);
                SetBkMode(hdc, OPAQUE as i32);
                return state.search_brush as isize;
            }
            0
        }
        WM_COMMAND => {
            if let Some(state) = get_state(hwnd) {
                let notify_code = ((wparam >> 16) & 0xFFFF) as u32;
                let ctrl_hwnd = lparam as HWND;
                if ctrl_hwnd == state.edit_hwnd && notify_code == EN_CHANGE {
                    refresh_filter(state);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        // Diffusé à tout le système dès qu'un process change le contenu du
        // presse-papier -- reçu ici tant que AddClipboardFormatListener est
        // enregistré sur ce hwnd (voir create()/toggle_copy_history).
        WM_CLIPBOARDUPDATE => {
            if let Some(state) = get_state(hwnd) {
                if state.suppress_next_clipboard_capture {
                    // Message déclenché par notre propre set_clipboard_text
                    // (re-copie d'une entrée de l'historique) : ignoré une
                    // fois, plutôt que de dupliquer l'entrée en tête.
                    state.suppress_next_clipboard_capture = false;
                } else if state.copy_history_enabled && !crate::win32::clipboard_excluded_from_history() {
                    // La source (gestionnaire de mots de passe le plus
                    // souvent) peut demander à ne pas être capturée par un
                    // surveillant de presse-papier. Vérifié à CHAQUE
                    // message, jamais mis en cache : le contenu précédent
                    // pouvait très bien ne pas être exclu.
                    if let Some(text) = get_clipboard_text(hwnd) {
                        state.copy_history.push(text);
                        if state.mode == Mode::CopyHistory {
                            rebuild_copy_history_items(state);
                            refresh_filter(state);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                }
            }
            0
        }
        WM_TIMER => {
            if let Some(state) = get_state(hwnd) {
                match wparam {
                    CLOCK_TIMER_ID => {
                        // Invalidation scopée au seul rect de l'horloge :
                        // ce tick tourne une fois par seconde tant que la
                        // fenêtre est ouverte, inutile de repeindre toute
                        // la scène pour un texte confiné à ce coin. Le repaint
                        // plein du décompte est à COUNTDOWN_TIMER_ID (même
                        // cadence, actif ⇔ `timer_deadline.is_some()`).
                        if state.theme.show_clock {
                            InvalidateRect(hwnd, &state.geometry.clock, 0);
                        }
                    }
                    COUNTDOWN_TIMER_ID => {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                        // Le tir n'est pas un timer à part : c'est ce battement
                        // qui, en franchissant `timer_deadline` (horloge
                        // monotone), enchaîne. Un doublon en file trouve None
                        // et ne fait que repeindre.
                        if let Some(deadline) = state.timer_deadline {
                            if std::time::Instant::now() >= deadline {
                                fire(hwnd, state);
                            }
                        }
                    }
                    BOUNCE_TIMER_ID => bounce_tick(hwnd, state),
                    RECYCLE_BIN_POLL_TIMER_ID => poll_recycle_bin(hwnd, state),
                    _ => {}
                }
            }
            0
        }
        // Un clic, quel que soit le bouton, est une des trois sorties du
        // rebond DVD (avec Échap et le raccourci global). La souris n'agit
        // nulle part ailleurs dans la fenêtre : hors rebond, ce bras est
        // un no-op.
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            if let Some(state) = get_state(hwnd) {
                if state.bouncing {
                    stop_bounce(hwnd, state);
                    hide(hwnd);
                }
            }
            0
        }
        // Alt+F4 (et tout SC_CLOSE) atteint la fenêtre par WM_CLOSE. Sans ce
        // bras, DefWindowProcW ferait DestroyWindow sur la fenêtre
        // PRINCIPALE, qui ne doit jamais être détruite avant la sortie du
        // process (seul tray_hwnd l'est, via "Quit"). L'appli resterait
        // alors à moitié vivante : WM_DESTROY libère l'état mais n'appelle
        // pas PostQuitMessage, donc le process survit avec un tray présent
        // et un hotkey pointant sur un HWND détruit. Alt+F4 est donc un
        // simple hide.
        // WM_CLOSE arrive par DispatchMessageW, sans passer par
        // l'interception WM_KEYDOWN de main.rs : il doit arrêter le rebond
        // lui-même, sinon la fenêtre se cache avec `bouncing` à true et
        // BOUNCE_TIMER_ID toujours armé, à repositionner une fenêtre
        // invisible jusqu'au prochain show().
        WM_CLOSE => {
            if let Some(state) = get_state(hwnd) {
                if state.bouncing {
                    stop_bounce(hwnd, state);
                }
            }
            hide(hwnd);
            0
        }
        WM_ACTIVATE => {
            // Perte de focus (bascule vers une autre appli) -> ferme la
            // popup.
            if (wparam & 0xFFFF) as u32 == WA_INACTIVE {
                if let Some(state) = get_state(hwnd) {
                    if state.bouncing {
                        stop_bounce(hwnd, state);
                    } else {
                        // Une preview de thème en cours doit être annulée
                        // ici aussi : sinon elle survit à une perte de
                        // focus (alt-tab, notification) et le prochain
                        // affichage garde les couleurs d'un thème jamais
                        // validé.
                        cancel_uncommitted_theme_preview(state);
                    }
                }
                // hide() plutôt qu'un ShowWindow(SW_HIDE) dupliqué ici : un
                // seul point d'entrée pour masquer la fenêtre, quel que soit
                // le déclencheur (Alt-Tab, Alt+F4, hotkey en bascule).
                hide(hwnd);
            }
            0
        }
        WM_DESTROY => {
            if let Some(state) = get_state(hwnd) {
                KillTimer(hwnd, CLOCK_TIMER_ID);
                KillTimer(hwnd, COUNTDOWN_TIMER_ID);
                KillTimer(hwnd, BOUNCE_TIMER_ID);
                KillTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID);
                state.process_supervisor.stop();
                if state.copy_history_enabled {
                    RemoveClipboardFormatListener(hwnd);
                }
                if !state.font_row.is_null() {
                    DeleteObject(state.font_row as _);
                }
                if !state.font_search.is_null() {
                    DeleteObject(state.font_search as _);
                }
                if !state.search_brush.is_null() {
                    DeleteObject(state.search_brush as _);
                }
                if !state.list_bg_brush.is_null() {
                    DeleteObject(state.list_bg_brush as _);
                }
                if !state.selected_bg_brush.is_null() {
                    DeleteObject(state.selected_bg_brush as _);
                }
                if !state.border_brush.is_null() {
                    DeleteObject(state.border_brush as _);
                }
                if !state.mem_bitmap.is_null() {
                    DeleteObject(state.mem_bitmap as _);
                }
                if !state.mem_dc.is_null() {
                    DeleteDC(state.mem_dc);
                }
            }
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Procédure d'origine de la classe stock "EDIT", capturée avant le
/// sous-classement (voir `create`). Tout EDIT de cette fenêtre partage la
/// même classe système, donc la même adresse : une capture suffit.
pub(crate) static ORIGINAL_EDIT_WNDPROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

unsafe fn call_original_edit_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let orig = ORIGINAL_EDIT_WNDPROC.load(std::sync::atomic::Ordering::Relaxed);
    let proc: WNDPROC = std::mem::transmute(orig);
    CallWindowProcW(proc, hwnd, msg, wparam, lparam)
}

/// Sous-classe du contrôle EDIT de recherche : l'appli est entièrement
/// pilotée au clavier, la souris ne doit rien pouvoir y faire. Filtrer les
/// touches dans `handle_edit_keydown` n'y suffit pas -- un EDIT natif
/// réagit aussi à la souris (caret, sélection, curseur I-beam au survol),
/// indépendamment du clavier. Tout message souris est donc avalé ici avant
/// d'atteindre la procédure d'origine ; le reste (texte, focus, police) lui
/// est transmis normalement.
pub(crate) unsafe extern "system" fn edit_subclass_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK
        | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_MOUSEMOVE | WM_MOUSEWHEEL | WM_NCLBUTTONDOWN => 0,
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_ARROW));
            1
        }
        // Restreint au champ vide, seul cas où le fond est peint ici (voir
        // WM_PAINT ci-dessous) : dès qu'il y a du texte,
        // call_original_edit_proc garde son propre WM_ERASEBKGND, dont il a
        // besoin pour effacer les anciens glyphes entre deux frappes.
        WM_ERASEBKGND if GetWindowTextLengthW(hwnd) == 0 => 1,
        // Champ vide : fond ET placeholder peints dans la MÊME session
        // BeginPaint/EndPaint plutôt qu'en deux présentations successives,
        // même principe d'atomicité que mem_dc dans AppState. La procédure
        // d'origine n'est donc jamais appelée pour ce WM_PAINT, d'où un
        // effet de bord : c'est normalement elle qui repositionne le caret
        // quand le texte change, responsabilité reprise ici tant que le
        // champ reste vide. SetCaretPos le replace au début du texte (même
        // text_margin que draw_placeholder) ; le Y n'a pas à être
        // recalculé, la boîte est mono-ligne. HideCaret/ShowCaret encadrent
        // le tout : le caret s'affiche en XOR, et peindre par-dessus sans
        // l'avoir caché corromprait le pixel qu'il inverse. Les deux sont
        // des no-op si aucun caret n'est affiché.
        WM_PAINT if GetWindowTextLengthW(hwnd) == 0 => {
            if let Some(state) = get_state(GetParent(hwnd)) {
                HideCaret(hwnd);
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT::default();
                GetClientRect(hwnd, &mut rc);
                FillRect(hdc, &rc, state.search_brush);
                draw_placeholder(hdc, hwnd, state);
                EndPaint(hwnd, &ps);
                let mut caret_pos = POINT::default();
                GetCaretPos(&mut caret_pos);
                SetCaretPos(state.text_margin, caret_pos.y);
                ShowCaret(hwnd);
                return 0;
            }
            call_original_edit_proc(hwnd, msg, wparam, lparam)
        }
        _ => call_original_edit_proc(hwnd, msg, wparam, lparam),
    }
}

/// Appelé par la boucle de messages AVANT TranslateMessage/DispatchMessage
/// pour tout WM_KEYDOWN destiné au contrôle EDIT -- `true` si la touche a
/// été traitée ici (ne doit PAS atteindre l'EDIT), `false` pour la laisser
/// suivre son chemin normal (saisie de texte, Ctrl+C...). Évite d'avoir à
/// sous-classer le contrôle EDIT pour un besoin aussi ciblé.
pub(crate) unsafe fn handle_edit_keydown(hwnd: HWND, vk: u16) -> bool {
    let Some(state) = get_state(hwnd) else { return false };

    // Pendant le rebond DVD, Échap suit la règle générale (voir plus bas) :
    // arrête le rebond et laisse la popup visible en mode Normal. Toute
    // autre touche la referme -- le rebond n'a aucun autre usage de la
    // saisie clavier.
    if state.bouncing {
        stop_bounce(hwnd, state);
        if vk == VK_ESCAPE {
            enter_mode(hwnd, state, Mode::Normal);
            SetFocus(state.edit_hwnd);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        } else {
            hide(hwnd);
        }
        return true;
    }

    let ctrl_down = (GetKeyState(VK_CONTROL as i32) as u16) & 0x8000 != 0;
    let shift_down = (GetKeyState(VK_SHIFT as i32) as u16) & 0x8000 != 0;

    // Ctrl+1..9/0 -> window_size 10%..100%, Ctrl+-/Ctrl+= -> bordure ∓1px.
    // Traités AVANT le remap Ctrl+S/W/D/A ci-dessous : ces touches n'ont
    // aucune signification hors Ctrl (un "1"/"-"/"=" seul reste une frappe
    // de recherche), ce ne sont donc pas des alias de touches existantes.
    if ctrl_down {
        let size_percent = match vk {
            VK_1 => Some(10),
            VK_2 => Some(20),
            VK_3 => Some(30),
            VK_4 => Some(40),
            VK_5 => Some(50),
            VK_6 => Some(60),
            VK_7 => Some(70),
            VK_8 => Some(80),
            VK_9 => Some(90),
            VK_0 => Some(100),
            _ => None,
        };
        if let Some(percent) = size_percent {
            set_window_size_percent(hwnd, state, percent);
            return true;
        }
        if vk == VK_OEM_MINUS {
            adjust_border(hwnd, state, -1);
            return true;
        }
        if vk == VK_OEM_PLUS {
            adjust_border(hwnd, state, 1);
            return true;
        }
    }

    // Ctrl+S/W/D/A : alias des flèches (navigation sans quitter le pavé de
    // lettres), normalisés ici plutôt que dupliqués dans le match ci-dessous.
    let vk = match (ctrl_down, vk) {
        (true, VK_S) => VK_DOWN,
        (true, VK_W) => VK_UP,
        (true, VK_D) => VK_RIGHT,
        (true, VK_A) => VK_LEFT,
        _ => vk,
    };

    // Chaque action ci-dessous est responsable de son propre repaint, et
    // seulement quand elle a réellement changé quelque chose
    // (move_selection/on_delete renvoient un bool en ce sens, enter_mode
    // s'invalide lui-même, hide() n'a rien à invalider). Pas de filet de
    // sécurité "on invalide au cas où" ici : il ferait redessiner toute la
    // fenêtre à chaque frappe d'une touche sans effet (Suppr sur liste
    // vide, Ctrl+1 déjà à 10%), source de flashs sous répétition rapide.
    let old_selected = state.selected;
    let old_first_visible = state.first_visible;

    match vk {
        // Haut/Bas d'une ligne, Gauche/Droite d'une page.
        VK_DOWN | VK_UP | VK_LEFT | VK_RIGHT => {
            let delta = match vk {
                VK_DOWN => 1,
                VK_UP => -1,
                VK_RIGHT => VISIBLE_ROWS as i32,
                _ => -(VISIBLE_ROWS as i32),
            };
            if super::items::move_selection(state, delta) {
                invalidate_after_navigation(hwnd, state, old_selected, old_first_visible);
            }
            true
        }
        VK_RETURN => {
            if shift_down {
                reveal_or_edit(hwnd, state);
            } else {
                launch_selected(hwnd, state);
            }
            true
        }
        // Règle unique quel que soit le mode : Échap ramène au menu
        // principal, et ferme la popup si on y est déjà. `exit_picker`
        // couvre tous les modes et annule une preview de thème non validée.
        VK_ESCAPE => {
            if state.mode == Mode::Normal {
                hide(hwnd);
            } else {
                exit_picker(hwnd, state);
            }
            true
        }
        // Règle unique quel que soit le mode : Tab va au Window Switcher.
        VK_TAB => {
            cancel_uncommitted_theme_preview(state);
            enter_mode(hwnd, state, Mode::Window);
            true
        }
        VK_DELETE => {
            if on_delete(hwnd, state, shift_down) {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            true
        }
        _ => false,
    }
}

/// N'invalide que les rects réellement affectés par un déplacement de
/// sélection (le cas de loin le plus fréquent, flèche maintenue), avec deux
/// retombées vers une invalidation plus large quand plus de deux lignes
/// changent : le défilement et le mode Thème.
unsafe fn invalidate_after_navigation(hwnd: HWND, state: &AppState, old_selected: usize, old_first_visible: usize) {
    if state.mode == Mode::Theme {
        // Chaque flèche prévisualise un thème différent, qui change aussi
        // search_background/border et pas seulement les couleurs des
        // lignes : rien de plus fin qu'un repaint plein-fenêtre ici.
        InvalidateRect(hwnd, std::ptr::null(), 0);
        return;
    }
    if state.first_visible != old_first_visible {
        // Défilement : toutes les lignes visibles changent de contenu
        // (chaque case affiche `first_visible + slot`), leur repaint est
        // donc nécessaire. Bordure, barre de recherche, horloge et
        // séparateur ne bougent jamais en scrollant : les exclure rend un
        // défilement continu nettement moins coûteux par frame.
        let band = rows_band_rect(&state.geometry);
        InvalidateRect(hwnd, &band, 0);
        return;
    }
    if old_selected >= old_first_visible && old_selected - old_first_visible < VISIBLE_ROWS {
        let r = state.geometry.rows[old_selected - old_first_visible];
        InvalidateRect(hwnd, &r, 0);
    }
    if state.selected >= state.first_visible && state.selected - state.first_visible < VISIBLE_ROWS {
        let r = state.geometry.rows[state.selected - state.first_visible];
        InvalidateRect(hwnd, &r, 0);
    }
}

/// Retour arrière sur une recherche déjà vide : sort du picker actif plutôt
/// que de ne rien faire. Point d'entrée distinct de handle_edit_keydown car
/// appelé avant Translate/Dispatch, quand le texte de l'EDIT n'a pas encore
/// changé.
pub(crate) unsafe fn handle_backspace_on_empty(hwnd: HWND) -> bool {
    let Some(state) = get_state(hwnd) else { return false };
    if state.mode == Mode::Normal {
        return false;
    }
    if !super::app_state::get_edit_text(state.edit_hwnd).is_empty() {
        return false;
    }
    exit_picker(hwnd, state);
    InvalidateRect(hwnd, std::ptr::null(), 0);
    true
}
