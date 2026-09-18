//! Changement de mode : entrée dans un mode (reconstruit `mode_items`,
//! réinitialise la recherche), et les deux sorties qui doivent annuler une
//! preview de thème non validée.

use crate::win32::gdi32::{InvalidateRect, RedrawWindow, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW};
use crate::win32::HWND;

use super::app_state::{refresh_placeholder, set_edit_text, AppState, Mode};
use super::geometry::VISIBLE_ROWS;
use super::items::{
    rebuild_copy_history_items, rebuild_eject_items, rebuild_emoji_items, rebuild_kill_items, rebuild_normal_items,
    rebuild_notes_items, rebuild_recycle_bin_items, rebuild_restart_items, rebuild_theme_items, rebuild_window_items,
    refresh_filter,
};
use crate::ui::theme;

pub(crate) unsafe fn enter_mode(hwnd: HWND, state: &mut AppState, mode: Mode) {
    if mode == Mode::Theme {
        state.theme_picker_original = Some(state.theme.active_theme.clone());
    }
    state.mode = mode;
    match mode {
        Mode::Normal => rebuild_normal_items(state),
        Mode::Window => rebuild_window_items(state),
        Mode::Notes => rebuild_notes_items(state),
        Mode::Restart => rebuild_restart_items(state),
        Mode::Kill => rebuild_kill_items(state),
        Mode::Theme => rebuild_theme_items(state),
        Mode::RecycleBin => rebuild_recycle_bin_items(hwnd, state),
        Mode::Emoji => rebuild_emoji_items(state),
        Mode::CopyHistory => rebuild_copy_history_items(state),
        Mode::Eject => rebuild_eject_items(state),
        Mode::Timer => {}
    }
    set_edit_text(state.edit_hwnd, "");
    refresh_placeholder(state);
    // RDW_UPDATENOW plutôt qu'un simple InvalidateRect, même raison que dans
    // apply_theme_visuals : enchaîner vite des changements de mode laisse
    // sinon l'ancien texte/placeholder à l'écran longtemps après que
    // set_edit_text("") ait vidé le contrôle, le WM_PAINT de l'EDIT ne
    // passant jamais devant les WM_KEYDOWN qui continuent d'arriver.
    RedrawWindow(state.edit_hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASE);
    refresh_filter(state);
    if mode == Mode::Theme {
        // refresh_filter remet la sélection à l'index 0 (premier thème dans
        // l'ordre alphabétique) : recalage sur le thème actif, pour que le
        // picker s'ouvre là où on est déjà.
        if let Some(idx) = state.mode_items.iter().position(|name| *name == state.theme.active_theme) {
            state.selected = idx;
            state.first_visible = if idx >= VISIBLE_ROWS { idx - VISIBLE_ROWS + 1 } else { 0 };
        }
    }
    // Un changement de mode change toujours le contenu affiché : invalidé
    // ici une fois pour toutes, plutôt que dans chacun des appelants. C'est
    // ce qui permet à handle_edit_keydown de n'invalider que sur changement
    // réel, sans filet de sécurité aveugle (voir son commentaire).
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

/// Restaure le thème actif d'origine si on quitte le sélecteur sans avoir
/// validé -- partagé par `exit_picker` (Échap) et Tab (le Window Switcher
/// est accessible depuis n'importe quel mode) : quitter Thème sans valider
/// ne doit jamais laisser un thème seulement prévisualisé.
pub(crate) unsafe fn cancel_uncommitted_theme_preview(state: &mut AppState) {
    if state.mode == Mode::Theme {
        if let Some(orig) = state.theme_picker_original.take() {
            theme::preview_theme(&mut state.theme, &orig);
            super::app_state::apply_theme_visuals(state);
        }
    }
}

/// Revient au mode Normal en annulant une preview de thème non validée.
pub(crate) unsafe fn exit_picker(hwnd: HWND, state: &mut AppState) {
    cancel_uncommitted_theme_preview(state);
    enter_mode(hwnd, state, Mode::Normal);
}
