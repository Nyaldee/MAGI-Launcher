//! Actions déclenchées par Entrée/Maj+Entrée/Suppr/Maj+Suppr, et Reload.

use crate::win32::gdi32::InvalidateRect;
use crate::win32::user32::{AddClipboardFormatListener, RemoveClipboardFormatListener};
use crate::win32::{set_clipboard_text, HWND};

use super::app_state::{get_edit_text, set_edit_text, AppState, Mode, SearchDisplay};
use super::geometry::apply_geometry;
use super::items::{
    eject_selected, rebuild_kill_items, rebuild_notes_items, rebuild_restart_items, refresh_filter,
    sync_recycle_bin_mode_items, sync_window_mode_items, SENTINEL_COPY_HISTORY, SENTINEL_EJECT, SENTINEL_EMOJI,
    SENTINEL_EMPTY_RECYCLE_BIN, SENTINEL_KILL, SENTINEL_MEDIA_NEXT, SENTINEL_MEDIA_PLAY_PAUSE, SENTINEL_MEDIA_PREVIOUS,
    SENTINEL_MEDIA_STOP, SENTINEL_MEDIA_VOLUME_DOWN, SENTINEL_MEDIA_VOLUME_MUTE, SENTINEL_MEDIA_VOLUME_UP,
    SENTINEL_NOTES, SENTINEL_OPEN_FOLDER, SENTINEL_RELOAD, SENTINEL_RESTART, SENTINEL_THEME_PICKER, SENTINEL_TIMER,
};
use super::hide;
use super::mode::{enter_mode, exit_picker};
use super::timer::{arm_timer, cancel_timer};
use crate::ui::theme;

/// Vide la Corbeille et rafraîchit le cache count/taille -- partagé par les
/// trois sites qui déclenchent un vidage complet (Maj+Entrée/Suppr sur la
/// ligne du menu principal, Maj+Suppr depuis la vue de consultation). Ne
/// ferme pas le lanceur : même convention que Suppr sur le Timer, une
/// action ponctuelle n'est pas une raison de quitter.
pub(crate) unsafe fn empty_recycle_bin(hwnd: HWND, state: &AppState) {
    crate::core::recycle_bin::empty_async();
    // Cache forcé à (0, 0) plutôt qu'invalidé : le vidage est asynchrone,
    // donc un `None` ferait re-interroger la Corbeille au tout prochain
    // repaint -- course quasi toujours perdue contre le thread de vidage à
    // peine lancé, dont le résultat "pas encore vide" resterait ensuite
    // figé pendant tout RECYCLE_BIN_CACHE_TTL. (0, 0) reflète l'intention
    // de l'action ; le TTL corrige de lui-même si le vidage échoue.
    state.recycle_bin_cache.set(Some((std::time::Instant::now(), 0, 0)));
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

/// Factorise "persister -> rebuild_*_items -> refresh_filter", répété à
/// l'identique dans launch_selected (Notes/Restart) et on_delete
/// (Notes/Restart/CopyHistory) -- seuls la sauvegarde (`persist`, no-op
/// pour CopyHistory qui ne touche jamais le disque) et le rebuild
/// (`rebuild`) changent d'un appel à l'autre. N'invalide pas elle-même :
/// chaque appelant le fait selon sa propre convention (voir on_delete/
/// launch_selected).
fn sync_list_mode(state: &mut AppState, persist: impl FnOnce(&mut AppState), rebuild: fn(&mut AppState)) {
    persist(state);
    rebuild(state);
    refresh_filter(state);
}

pub(crate) unsafe fn reload_config(hwnd: HWND, state: &mut AppState) {
    if let Ok(cfg) = crate::core::config::load_all(&state.base_dir) {
        state.apps = cfg.apps;
    }
    state.themes_json_present = theme::load(&state.themes_path, &mut state.theme);
    let fresh_state = crate::core::state::load(&state.state_path);
    theme::apply_prefs(&mut state.theme, &fresh_state.ui);
    // Appliqué avec les mêmes effets de bord que toggle_auto_restart/
    // toggle_copy_history (démarrer/arrêter le superviseur, (dés)enregistrer
    // le listener presse-papier) -- une simple affectation du booléen
    // laisserait l'état réel (superviseur tournant, listener enregistré)
    // désynchronisé de la valeur affichée/relue au prochain Reload.
    if fresh_state.auto_restart_enabled != state.auto_restart_enabled {
        state.auto_restart_enabled = fresh_state.auto_restart_enabled;
        state.process_supervisor.set_restart_enabled(state.auto_restart_enabled);
    }
    if fresh_state.auto_kill_enabled != state.auto_kill_enabled {
        state.auto_kill_enabled = fresh_state.auto_kill_enabled;
        // Réactiver via Reload (édition manuelle de state.json) (ré)arme le
        // délai de grâce, comme la bascule du tray.
        state.process_supervisor.set_kill_enabled(state.auto_kill_enabled);
    }
    if fresh_state.copy_history_enabled != state.copy_history_enabled {
        state.copy_history_enabled = fresh_state.copy_history_enabled;
        if state.copy_history_enabled {
            AddClipboardFormatListener(hwnd);
        } else {
            RemoveClipboardFormatListener(hwnd);
        }
    }
    if let Some(hook) = state.on_hotkey_reload.as_mut() {
        hook(&fresh_state.hotkey, fresh_state.hotkey_enabled);
    }
    // notes.json/restart.json sont normalement déjà à jour en mémoire (le
    // lanceur est seul à les écrire, voir core::json_list) -- les relire
    // coûte peu et couvre une édition manuelle faite pendant que le lanceur
    // tourne, sinon invisible jusqu'au prochain redémarrage.
    state.notes = crate::core::json_list::load_notes(&state.notes_path);
    state.restart_targets = crate::core::json_list::load_restart_list(&state.restart_path);
    state.process_supervisor.set_restart_targets(state.restart_targets.clone());
    state.kill_targets = crate::core::json_list::load_kill_list(&state.kill_path);
    state.process_supervisor.set_kill_targets(state.kill_targets.clone());
    // Même logique pour emoji-test.txt : permet de déposer une version plus
    // récente d'Unicode sans redémarrer le lanceur.
    state.emoji = crate::core::emoji::load(&state.base_dir.join("emoji-test.txt"));
    apply_geometry(hwnd, state);
    enter_mode(hwnd, state, Mode::Normal);
}

pub(crate) unsafe fn launch_selected(hwnd: HWND, state: &mut AppState) {
    match state.mode {
        Mode::Normal => launch_selected_normal(hwnd, state),
        Mode::Window => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(w) = state.windows.get(idx) {
                    crate::core::windows::activate_window(w.hwnd);
                }
            }
            hide(hwnd);
        }
        Mode::Notes => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                // Correspondance existante -> copie et ferme.
                if let Some(note) = state.notes.get(idx).cloned() {
                    set_clipboard_text(hwnd, &note);
                    hide(hwnd);
                    return;
                }
            }
            let query = get_edit_text(state.edit_hwnd);
            if !query.trim().is_empty() {
                state.notes.insert(0, query);
                set_edit_text(state.edit_hwnd, "");
                sync_list_mode(
                    state,
                    |s| {
                        let _ = crate::core::json_list::save_notes(&s.notes_path, &s.notes);
                    },
                    rebuild_notes_items,
                );
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Mode::Restart => {
            if !state.filtered.is_empty() {
                return; // cible déjà surveillée -- rien à faire ici (voir Suppr)
            }
            let query = get_edit_text(state.edit_hwnd);
            let target = query.trim().to_string();
            // Aucune validation de format : toute cible non vide est
            // acceptée telle quelle, arguments compris. Une cible invalide
            // se voit à l'usage plutôt que d'être rejetée a priori.
            if !target.is_empty() {
                state.restart_targets.push(target);
                set_edit_text(state.edit_hwnd, "");
                sync_list_mode(
                    state,
                    |s| {
                        let _ = crate::core::json_list::save_restart_list(&s.restart_path, &s.restart_targets);
                        s.process_supervisor.set_restart_targets(s.restart_targets.clone());
                    },
                    rebuild_restart_items,
                );
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Mode::Kill => {
            if !state.filtered.is_empty() {
                return; // cible déjà dans la liste -- rien à faire ici (voir Suppr)
            }
            let target = get_edit_text(state.edit_hwnd).trim().to_string();
            if !target.is_empty() {
                state.kill_targets.push(target);
                set_edit_text(state.edit_hwnd, "");
                sync_list_mode(
                    state,
                    |s| {
                        let _ = crate::core::json_list::save_kill_list(&s.kill_path, &s.kill_targets);
                        s.process_supervisor.set_kill_targets(s.kill_targets.clone());
                    },
                    rebuild_kill_items,
                );
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Mode::Theme => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(name) = state.mode_items.get(idx).cloned() {
                    theme::preview_theme(&mut state.theme, &name);
                    let _ = crate::core::state::commit_theme(&state.state_path, &name);
                    state.theme.active_theme = name;
                    state.theme_picker_original = None;
                    super::app_state::apply_theme_visuals(state);
                }
            }
            enter_mode(hwnd, state, Mode::Normal);
        }
        Mode::Timer => {
            if let Some(secs) = crate::core::timer::parse_duration(get_edit_text(state.edit_hwnd).trim()) {
                arm_timer(hwnd, state, secs);
                exit_picker(hwnd, state);
            }
        }
        // Copie le nom complet (extension comprise) de l'élément
        // sélectionné. Vider la Corbeille reste réservé à Maj+Suppr ici, ou
        // à Maj+Entrée/Suppr sur "Empty Recycle Bin" au menu principal.
        Mode::RecycleBin => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(item) = state.recycle_bin_items.get(idx) {
                    set_clipboard_text(hwnd, &item.name);
                    hide(hwnd);
                }
            }
        }
        Mode::Emoji => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(entry) = state.emoji.as_ref().and_then(|d| d.entries.get(idx)) {
                    set_clipboard_text(hwnd, &entry.glyph);
                    hide(hwnd);
                }
            }
        }
        // Ne ferme pas le lanceur, contrairement aux autres modes : plusieurs
        // périphériques branchés à la fois est courant, autant pouvoir les
        // éjecter à la suite. `force = false` : si un handle est encore
        // ouvert sur le volume (FSCTL_LOCK_VOLUME refusé), Entrée n'insiste
        // pas et l'entrée reste dans la liste -- forcer reste un geste
        // distinct (Maj+Suppr, voir on_delete).
        Mode::Eject => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if eject_selected(state, idx, false) {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
        }
        // Pas d'ajout à la saisie ici, contrairement à Sticky Notes : les
        // entrées viennent uniquement de la capture presse-papier (voir
        // WM_CLIPBOARDUPDATE).
        Mode::CopyHistory => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(text) = state.copy_history.get(idx).map(str::to_string) {
                    // Posé AVANT set_clipboard_text : le WM_CLIPBOARDUPDATE
                    // que cette copie va déclencher doit être ignoré, sinon
                    // l'entrée serait réinjectée en doublon en tête.
                    state.suppress_next_clipboard_capture = true;
                    set_clipboard_text(hwnd, &text);
                    hide(hwnd);
                }
            }
        }
    }
}

pub(crate) unsafe fn launch_selected_normal(hwnd: HWND, state: &mut AppState) {
    match &state.display {
        SearchDisplay::Calc(text) => {
            let value = text.trim_start_matches("= ").to_string();
            set_clipboard_text(hwnd, &value);
            hide(hwnd);
        }
        SearchDisplay::Color(_) => {
            let query = get_edit_text(state.edit_hwnd);
            set_clipboard_text(hwnd, query.trim());
            hide(hwnd);
        }
        SearchDisplay::SingleLine(_) => {}
        SearchDisplay::List => {
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(app) = state.apps.get(idx) else { return };
            let path = app.path.as_str();
            match path {
                SENTINEL_RELOAD => {
                    reload_config(hwnd, state);
                    hide(hwnd);
                }
                // Rien si themes.json est absent/invalide : l'entrée
                // l'annonce déjà (row_label : "Theme: missing themes.json"),
                // inutile d'ouvrir un sélecteur réduit au thème de secours.
                SENTINEL_THEME_PICKER => {
                    if state.themes_json_present {
                        enter_mode(hwnd, state, Mode::Theme);
                    }
                }
                SENTINEL_TIMER => enter_mode(hwnd, state, Mode::Timer),
                SENTINEL_NOTES => enter_mode(hwnd, state, Mode::Notes),
                SENTINEL_RESTART => enter_mode(hwnd, state, Mode::Restart),
                SENTINEL_KILL => enter_mode(hwnd, state, Mode::Kill),
                // Rien si emoji-test.txt est absent : l'entrée l'annonce
                // déjà (row_label : "Emoji: missing emoji-test.txt").
                SENTINEL_EMOJI => {
                    if state.emoji.is_some() {
                        enter_mode(hwnd, state, Mode::Emoji);
                    }
                }
                // Rien si désactivé -- l'entrée l'annonce déjà (voir
                // row_label : "Copy History: disabled"), à activer depuis
                // le menu tray plutôt que depuis ce picker.
                SENTINEL_COPY_HISTORY => {
                    if state.copy_history_enabled {
                        enter_mode(hwnd, state, Mode::CopyHistory);
                    }
                }
                SENTINEL_OPEN_FOLDER => {
                    let _ = crate::core::launch::launch(&state.base_dir.to_string_lossy(), None, false);
                    hide(hwnd);
                }
                // Entrée ouvre la vue de consultation (lecture seule) ;
                // Maj+Entrée (reveal_or_edit) vide la Corbeille -- l'action
                // destructive est réservée au modificateur.
                SENTINEL_EMPTY_RECYCLE_BIN => enter_mode(hwnd, state, Mode::RecycleBin),
                SENTINEL_EJECT => enter_mode(hwnd, state, Mode::Eject),
                SENTINEL_MEDIA_PLAY_PAUSE => send_media(hwnd, crate::core::media::MediaKey::PlayPause),
                SENTINEL_MEDIA_NEXT => send_media(hwnd, crate::core::media::MediaKey::Next),
                SENTINEL_MEDIA_PREVIOUS => send_media(hwnd, crate::core::media::MediaKey::Previous),
                SENTINEL_MEDIA_STOP => send_media(hwnd, crate::core::media::MediaKey::Stop),
                SENTINEL_MEDIA_VOLUME_MUTE => send_media(hwnd, crate::core::media::MediaKey::VolumeMute),
                SENTINEL_MEDIA_VOLUME_DOWN => send_media(hwnd, crate::core::media::MediaKey::VolumeDown),
                SENTINEL_MEDIA_VOLUME_UP => send_media(hwnd, crate::core::media::MediaKey::VolumeUp),
                _ => {
                    let path = app.path.clone();
                    let cwd = app.cwd.clone();
                    let hidden = app.hidden;
                    hide(hwnd);
                    let _ = crate::core::launch::launch(&path, cwd.as_deref(), hidden);
                }
            }
        }
    }
}

unsafe fn send_media(hwnd: HWND, key: crate::core::media::MediaKey) {
    crate::core::media::send_media_key(key);
    hide(hwnd);
}

/// Maj+Entrée : révèle la cible dans l'Explorateur au lieu de la lancer
/// (mode Normal), ou ouvre notes.json dans son éditeur associé (mode Notes).
pub(crate) unsafe fn reveal_or_edit(hwnd: HWND, state: &mut AppState) {
    match state.mode {
        Mode::Notes => {
            let _ = crate::core::launch::launch(&state.notes_path.to_string_lossy(), None, false);
            hide(hwnd);
        }
        Mode::Normal => {
            let SearchDisplay::List = state.display else { return };
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(app) = state.apps.get(idx) else { return };
            if app.path == SENTINEL_EMPTY_RECYCLE_BIN {
                empty_recycle_bin(hwnd, state);
                return;
            }
            if !app.path.starts_with("magi:") {
                let path = app.path.clone();
                hide(hwnd);
                let _ = crate::core::launch::reveal_in_explorer(&path);
            }
        }
        _ => {}
    }
}

/// `true` si Suppr/Maj+Suppr a réellement changé quelque chose, `false`
/// pour un no-op (rien de sélectionné, liste vide, mode où Suppr n'a pas
/// de sens). N'invalide jamais elle-même : l'appelant le fait une fois, une
/// fois la réponse connue -- sinon Suppr maintenue redessine la fenêtre à
/// chaque frappe pour un résultat identique.
pub(crate) unsafe fn on_delete(hwnd: HWND, state: &mut AppState, shift: bool) -> bool {
    match state.mode {
        Mode::Window => {
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            let Some(w) = state.windows.get(idx) else { return false };
            if shift {
                crate::core::windows::kill_window(w.hwnd);
            } else {
                crate::core::windows::close_window(w.hwnd);
            }
            // Suppression optimiste de la liste locale plutôt qu'une
            // ré-énumération : close_window passe par
            // PostMessageW(WM_CLOSE), asynchrone -- un EnumWindows immédiat
            // retrouve presque toujours la fenêtre encore vivante, et Suppr
            // paraîtrait alors sans effet.
            state.windows.remove(idx);
            sync_window_mode_items(state);
            refresh_filter(state);
            true
        }
        Mode::Notes => {
            let changed = if shift {
                let had_notes = !state.notes.is_empty();
                state.notes.clear();
                had_notes
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.notes.len() {
                    state.notes.remove(idx);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !changed {
                return false;
            }
            sync_list_mode(
                state,
                |s| {
                    let _ = crate::core::json_list::save_notes(&s.notes_path, &s.notes);
                },
                rebuild_notes_items,
            );
            true
        }
        Mode::CopyHistory => {
            let changed = if shift {
                let had_entries = state.copy_history.len() > 0;
                state.copy_history.clear();
                had_entries
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                state.copy_history.remove(idx);
                true
            } else {
                false
            };
            if !changed {
                return false;
            }
            sync_list_mode(state, |_| {}, super::items::rebuild_copy_history_items);
            true
        }
        Mode::Restart => {
            let changed = if shift {
                let had_targets = !state.restart_targets.is_empty();
                state.restart_targets.clear();
                had_targets
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.restart_targets.len() {
                    state.restart_targets.remove(idx);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !changed {
                return false;
            }
            sync_list_mode(
                state,
                |s| {
                    let _ = crate::core::json_list::save_restart_list(&s.restart_path, &s.restart_targets);
                    s.process_supervisor.set_restart_targets(s.restart_targets.clone());
                },
                rebuild_restart_items,
            );
            true
        }
        Mode::Kill => {
            let changed = if shift {
                let had_targets = !state.kill_targets.is_empty();
                state.kill_targets.clear();
                had_targets
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.kill_targets.len() {
                    state.kill_targets.remove(idx);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !changed {
                return false;
            }
            sync_list_mode(
                state,
                |s| {
                    let _ = crate::core::json_list::save_kill_list(&s.kill_path, &s.kill_targets);
                    s.process_supervisor.set_kill_targets(s.kill_targets.clone());
                },
                rebuild_kill_items,
            );
            true
        }
        Mode::RecycleBin => {
            if shift {
                empty_recycle_bin(hwnd, state);
                // Le vidage est asynchrone, mais le résultat est certain :
                // inutile d'attendre un nouveau scan pour que la vue de
                // consultation le reflète.
                state.recycle_bin_items.clear();
                state.mode_items.clear();
                refresh_filter(state);
                return true;
            }
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            let Some(item) = state.recycle_bin_items.get(idx).cloned() else { return false };
            crate::core::recycle_bin::delete_item(&item);
            state.recycle_bin_cache.set(None);
            // Suppression optimiste de la liste locale plutôt qu'un nouveau
            // scan complet (même principe que Mode::Window) : évite de
            // vider la vue le temps qu'un scan en arrière-plan revienne,
            // pour un seul élément déjà supprimé avec certitude.
            state.recycle_bin_items.remove(idx);
            sync_recycle_bin_mode_items(state);
            refresh_filter(state);
            true
        }
        Mode::Timer => {
            if state.timer_deadline.is_some() {
                cancel_timer(hwnd, state);
                true
            } else {
                false
            }
        }
        // Suppr seul ne fait rien : Entrée est déjà l'action normale du
        // mode. Maj+Suppr force le démontage même si le volume est encore
        // utilisé (voir disk_ejector::eject_drive), geste volontairement
        // distinct d'Entrée.
        Mode::Eject => {
            if !shift {
                return false;
            }
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            eject_selected(state, idx, true)
        }
        Mode::Normal => {
            let SearchDisplay::List = state.display else { return false };
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            let Some(app) = state.apps.get(idx) else { return false };
            if app.path == SENTINEL_EMPTY_RECYCLE_BIN {
                empty_recycle_bin(hwnd, state);
                true
            } else if app.path == SENTINEL_TIMER && state.timer_deadline.is_some() {
                cancel_timer(hwnd, state);
                true
            } else if app.path == SENTINEL_NOTES && !state.notes.is_empty() {
                // Retire la note la plus récente (notes[0], même convention
                // que row_label) sans entrer dans le picker Notes -- même
                // esprit que Suppr sur le Timer juste au-dessus.
                state.notes.remove(0);
                let _ = crate::core::json_list::save_notes(&state.notes_path, &state.notes);
                true
            } else if app.path == SENTINEL_COPY_HISTORY && state.copy_history.len() > 0 {
                // Même principe que SENTINEL_NOTES : retire l'entrée la
                // plus récente (index 0) sans entrer dans le picker.
                state.copy_history.remove(0);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}
