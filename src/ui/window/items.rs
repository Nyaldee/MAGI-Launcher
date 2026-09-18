//! Construction des libellés par mode (tous les `rebuild_*_items`) et
//! filtrage. Tous les modes partagent la même primitive de liste :
//! `mode_items` (les libellés à filtrer/afficher) + `filtered` (les indices
//! retenus, triés) + `selected`/`first_visible` -- un seul chemin de
//! rendu/filtrage plutôt que cinq quasi-identiques.

use std::time::Instant;

use crate::core::calculator;
use crate::core::disk_ejector::EjectableDrive;
use crate::core::search::{match_rank_multi, normalize};
use crate::win32::gdi32::InvalidateRect;
use crate::win32::user32::SetTimer;
use crate::win32::HWND;

use crate::ui::theme;

use super::app_state::{apply_theme_visuals, get_edit_text, recycle_bin_cached, AppState, Mode, SearchDisplay};
use super::geometry::VISIBLE_ROWS;
use super::timer::RECYCLE_BIN_POLL_TIMER_ID;

// Entrées spéciales de apps.json (voir README "Special entries").
pub(crate) const SENTINEL_RELOAD: &str = "magi:reload";
pub(crate) const SENTINEL_THEME_PICKER: &str = "magi:theme-picker";
pub(crate) const SENTINEL_TIMER: &str = "magi:timer";
pub(crate) const SENTINEL_NOTES: &str = "magi:notes";
pub(crate) const SENTINEL_RESTART: &str = "magi:auto-restart";
pub(crate) const SENTINEL_KILL: &str = "magi:auto-kill";
pub(crate) const SENTINEL_OPEN_FOLDER: &str = "magi:open-folder";
pub(crate) const SENTINEL_EMPTY_RECYCLE_BIN: &str = "magi:empty-recycle-bin";
pub(crate) const SENTINEL_MEDIA_PLAY_PAUSE: &str = "magi:media-play-pause";
pub(crate) const SENTINEL_MEDIA_NEXT: &str = "magi:media-next";
pub(crate) const SENTINEL_MEDIA_PREVIOUS: &str = "magi:media-previous";
pub(crate) const SENTINEL_MEDIA_STOP: &str = "magi:media-stop";
pub(crate) const SENTINEL_MEDIA_VOLUME_MUTE: &str = "magi:media-volume-mute";
pub(crate) const SENTINEL_MEDIA_VOLUME_DOWN: &str = "magi:media-volume-down";
pub(crate) const SENTINEL_MEDIA_VOLUME_UP: &str = "magi:media-volume-up";
pub(crate) const SENTINEL_EMOJI: &str = "magi:emoji";
pub(crate) const SENTINEL_COPY_HISTORY: &str = "magi:copy-history";
pub(crate) const SENTINEL_EJECT: &str = "magi:eject";

/// Cadence de sondage du résultat du scan Corbeille en arrière-plan (voir
/// rebuild_recycle_bin_items) -- assez court pour que la liste apparaisse
/// sans délai perceptible une fois le scan terminé, sans pomper le thread
/// UI pour rien entre-temps.
const RECYCLE_BIN_POLL_INTERVAL_MS: u32 = 80;

pub(crate) fn rebuild_normal_items(state: &mut AppState) {
    state.mode_items = state.apps.iter().map(|a| a.name.clone()).collect();
}

pub(crate) fn rebuild_notes_items(state: &mut AppState) {
    state.mode_items = state.notes.clone();
}

pub(crate) fn rebuild_restart_items(state: &mut AppState) {
    // Instantané tenu par le superviseur (voir running_names_snapshot) --
    // pas de CreateToolhelp32Snapshot sur le thread UI.
    let running = state.process_supervisor.running_names_snapshot();
    state.mode_items = state
        .restart_targets
        .iter()
        .map(|t| {
            let name = crate::core::supervisor::exe_basename(t);
            let marker = if running.contains(&name) { '\u{2605}' } else { '\u{2606}' };
            format!("{} {}", marker, t)
        })
        .collect();
}

/// Miroir de `rebuild_restart_items` : ✖ = cible en cours (sera terminée
/// après le délai de grâce), · = pas en cours.
pub(crate) fn rebuild_kill_items(state: &mut AppState) {
    let running = state.process_supervisor.running_names_snapshot();
    state.mode_items = state
        .kill_targets
        .iter()
        .map(|t| {
            let name = crate::core::supervisor::exe_basename(t);
            let marker = if running.contains(&name) { '\u{2716}' } else { '\u{00B7}' };
            format!("{} {}", marker, t)
        })
        .collect();
}

/// Lance le scan de la Corbeille -- une fois à l'entrée du mode, pas à
/// chaque frappe. `list_items()` énumère TOUS les lecteurs sur disque (I/O
/// potentiellement lente), donc déporté sur un thread dédié comme
/// `recycle_bin::empty_async()` : le résultat arrive via
/// `recycle_bin_pending`, sondé par RECYCLE_BIN_POLL_TIMER_ID. La liste
/// démarre vide et se peuple au retour du scan, sans bloquer l'affichage.
pub(crate) unsafe fn rebuild_recycle_bin_items(hwnd: HWND, state: &mut AppState) {
    state.recycle_bin_items.clear();
    state.mode_items.clear();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(crate::core::recycle_bin::list_items());
    });
    state.recycle_bin_pending = Some(rx);
    SetTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID, RECYCLE_BIN_POLL_INTERVAL_MS, None);
}

/// Sondé par WM_TIMER(RECYCLE_BIN_POLL_TIMER_ID) -- récupère le résultat du
/// scan dès qu'il est prêt. Met toujours à jour `recycle_bin_items` (lu
/// aussi hors de ce mode, voir row_label/on_delete), mais ne touche
/// `mode_items`/le filtre/le rendu que si on est ENCORE dans
/// Mode::RecycleBin : sans cette garde, un scan qui revient après une
/// sortie du mode écraserait la liste du mode devenu actif.
pub(crate) unsafe fn poll_recycle_bin(hwnd: HWND, state: &mut AppState) {
    let Some(rx) = &state.recycle_bin_pending else { return };
    match rx.try_recv() {
        Ok(items) => {
            state.recycle_bin_items = items;
            state.recycle_bin_pending = None;
            crate::win32::user32::KillTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID);
            if state.mode == Mode::RecycleBin {
                sync_recycle_bin_mode_items(state);
                refresh_filter(state);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            state.recycle_bin_pending = None;
            crate::win32::user32::KillTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID);
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {}
    }
}

/// Libellés du mode Corbeille à partir de `recycle_bin_items` -- partagé
/// par le retour de scan (poll_recycle_bin) et la suppression optimiste
/// d'un élément (on_delete).
pub(crate) fn sync_recycle_bin_mode_items(state: &mut AppState) {
    state.mode_items = state.recycle_bin_items.iter().map(|item| item.name.clone()).collect();
}

/// Libellés du Window Switcher à partir de `windows` -- partagé par la
/// (ré)énumération et la suppression optimiste d'une entrée (on_delete).
pub(crate) fn sync_window_mode_items(state: &mut AppState) {
    state.mode_items = state.windows.iter().map(|w| w.title.clone()).collect();
}

pub(crate) fn rebuild_window_items(state: &mut AppState) {
    state.windows = crate::core::windows::list_windows();
    sync_window_mode_items(state);
}

/// Synchrone, contrairement à rebuild_recycle_bin_items :
/// `list_ejectable_drives` ne touche qu'une poignée de lettres de lecteur
/// avec quelques IOCTL chacune, rien qui justifie un thread dédié.
pub(crate) fn rebuild_eject_items(state: &mut AppState) {
    state.eject_drives = crate::core::disk_ejector::list_ejectable_drives();
    state.mode_items = state.eject_drives.iter().map(eject_drive_label).collect();
}

fn eject_drive_label(drive: &EjectableDrive) -> String {
    if drive.label.is_empty() {
        format!("{}:  Removable Disk", drive.letter)
    } else {
        format!("{}:  {}", drive.letter, drive.label)
    }
}

/// Partagé par Entrée (`force = false`) et Maj+Suppr (`force = true`) --
/// seul `force` change, la mise à jour de la liste locale en cas de succès
/// est identique. N'invalide pas le rendu (même convention que les branches
/// de on_delete) : à l'appelant de le faire une fois la réponse connue.
pub(crate) fn eject_selected(state: &mut AppState, idx: usize, force: bool) -> bool {
    let Some(drive) = state.eject_drives.get(idx).cloned() else { return false };
    if !crate::core::disk_ejector::eject_drive(drive.letter, force) {
        return false;
    }
    state.eject_drives.remove(idx);
    state.mode_items = state.eject_drives.iter().map(eject_drive_label).collect();
    refresh_filter(state);
    true
}

pub(crate) fn rebuild_theme_items(state: &mut AppState) {
    state.mode_items = theme::list_theme_names(&state.theme);
}

pub(crate) fn rebuild_emoji_items(state: &mut AppState) {
    state.mode_items =
        state.emoji.as_ref().map(|d| d.entries.iter().map(|e| e.name.clone()).collect()).unwrap_or_default();
}

/// Contenu de l'historique presse-papier -- déjà entièrement en RAM
/// (core::clipboard_history), aucune I/O à déporter ici.
pub(crate) fn rebuild_copy_history_items(state: &mut AppState) {
    state.mode_items = (0..state.copy_history.len()).filter_map(|i| state.copy_history.get(i).map(str::to_string)).collect();
}

/// "<nom>: <valeur>" -- moule commun aux branches de `row_label`, qui ne
/// diffèrent que par la valeur affichée.
fn suffixed(name: &str, value: impl std::fmt::Display) -> String {
    format!("{name}: {value}")
}

/// Libellé affiché pour l'entrée `idx` du mode courant -- dérive de
/// `mode_items`, sauf pour les entrées du mode Normal qui affichent une
/// valeur recalculée à chaque rendu (compte de la Corbeille, compte à
/// rebours du Timer, note la plus récente...).
pub(crate) fn row_label(state: &AppState, idx: usize) -> String {
    if state.mode == Mode::Normal {
        if let Some(app) = state.apps.get(idx) {
            // Suffixe "<nom>: ..." même à l'état vide : une entrée qui
            // affiche tantôt son nom brut, tantôt "nom: état", laisserait
            // croire qu'elle ne fait jamais rien. Seule la Corbeille déroge
            // à la règle -- à l'état vide, "Empty Recycle Bin" seul reste
            // plus clair qu'un "Empty Recycle Bin: " sans valeur.
            match app.path.as_str() {
                SENTINEL_EMPTY_RECYCLE_BIN => {
                    let (count, size) = recycle_bin_cached(state);
                    return if count > 0 {
                        format!("{}: {} items, {:.1} MB", app.name, count, size as f64 / 1_048_576.0)
                    } else {
                        app.name.clone()
                    };
                }
                SENTINEL_TIMER => {
                    let value = match state.timer_deadline {
                        Some(deadline) => {
                            let remaining = (deadline - Instant::now()).as_secs() as i64;
                            crate::core::timer::format_remaining(remaining)
                        }
                        None => "--:--".to_string(),
                    };
                    return suffixed(&app.name, value);
                }
                // La plus récente en premier (voir launch_selected :
                // insert(0, ..)), donc notes[0] est bien la dernière ajoutée.
                SENTINEL_NOTES => {
                    return match state.notes.first() {
                        Some(latest) => suffixed(&app.name, latest),
                        None => format!("{}:", app.name),
                    };
                }
                SENTINEL_RESTART => return suffixed(&app.name, state.restart_targets.len()),
                SENTINEL_KILL => return suffixed(&app.name, state.kill_targets.len()),
                SENTINEL_THEME_PICKER => {
                    let value =
                        if state.themes_json_present { state.theme.active_theme.clone() } else { "missing themes.json".to_string() };
                    return suffixed(&app.name, value);
                }
                SENTINEL_EMOJI => {
                    let status = match &state.emoji {
                        Some(data) => format!("Version {}", data.version),
                        None => "missing emoji-test.txt".to_string(),
                    };
                    return suffixed(&app.name, status);
                }
                SENTINEL_COPY_HISTORY => {
                    let status =
                        if state.copy_history_enabled { state.copy_history.len().to_string() } else { "disabled".to_string() };
                    return suffixed(&app.name, status);
                }
                _ => {}
            }
        }
    }
    // "<emoji> <nom>" à l'affichage seulement : `mode_items` (la chaîne
    // filtrée) ne contient QUE le nom. Avec le glyphe en préfixe, "gri" ne
    // matcherait plus "grinning face" en tête (tier 0), la comparaison
    // commençant alors par l'emoji.
    if state.mode == Mode::Emoji {
        if let Some(entry) = state.emoji.as_ref().and_then(|d| d.entries.get(idx)) {
            return format!("{} {}", entry.glyph, entry.name);
        }
    }
    state.mode_items.get(idx).cloned().unwrap_or_default()
}

// --- Filtrage -------------------------------------------------------------

/// `normalized_items` déjà repliés/minuscules -- la normalisation est le
/// coût réel ici (une allocation par élément), faite une fois par
/// changement de liste et non à chaque frappe (voir normalized_mode_items).
fn fuzzy_filter(normalized_items: &[String], query_lower: &str) -> Vec<usize> {
    if query_lower.is_empty() {
        return (0..normalized_items.len()).collect();
    }
    let mut ranked: Vec<(usize, (u8, usize))> = normalized_items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match_rank_multi(s, query_lower).map(|r| (i, r)))
        .collect();
    ranked.sort_by_key(|&(_, rank)| rank);
    ranked.into_iter().map(|(i, _)| i).collect()
}

/// Version normalisée de `state.mode_items`, recalculée seulement si
/// `mode_items` a changé depuis le dernier appel (voir `mode_items_cache`).
fn normalized_mode_items(state: &mut AppState) -> &[String] {
    if state.mode_items_cache.0 != state.mode_items {
        state.mode_items_cache.1 = state.mode_items.iter().map(|s| normalize(s)).collect();
        state.mode_items_cache.0 = state.mode_items.clone();
    }
    &state.mode_items_cache.1
}

/// Réévalue le mode d'affichage et le classement à partir du texte actuel
/// de l'EDIT. En mode Normal, même ordre de priorité que le README :
/// couleur hex, puis expression arithmétique, puis recherche floue.
pub(crate) fn refresh_filter(state: &mut AppState) {
    let query = get_edit_text(state.edit_hwnd);
    let trimmed = query.trim();
    state.selected = 0;
    state.first_visible = 0;

    if state.mode == Mode::Timer {
        state.display = SearchDisplay::SingleLine(match crate::core::timer::parse_duration(trimmed) {
            Some(secs) => format!("Timer: {}", crate::core::timer::format_remaining(secs as i64)),
            None => "Timer: --:--".to_string(),
        });
        state.filtered.clear();
        return;
    }

    if state.mode == Mode::Normal {
        if let Some(color) = theme::parse_hex_color(trimmed) {
            state.display = SearchDisplay::Color(color);
            state.filtered.clear();
            return;
        }
        if calculator::looks_like_expression(trimmed) {
            if let Some(v) = calculator::evaluate(trimmed) {
                state.display = SearchDisplay::Calc(format!("= {}", calculator::format_result(v)));
                state.filtered.clear();
                return;
            }
        }
    }

    state.display = SearchDisplay::List;
    let query_normalized = normalize(trimmed);
    state.filtered = fuzzy_filter(normalized_mode_items(state), &query_normalized);
}

pub(crate) fn current_list_len(state: &AppState) -> usize {
    match state.display {
        SearchDisplay::List => state.filtered.len(),
        _ => 0,
    }
}

/// `true` si la sélection (ou le défilement) a réellement bougé, `false`
/// pour un no-op (liste vide, ou déjà à la butée dans la direction
/// demandée). Le retour pilote l'invalidation côté appelant : sans lui,
/// une flèche maintenue contre une butée redessine la fenêtre en boucle
/// pour un résultat identique.
pub(crate) unsafe fn move_selection(state: &mut AppState, delta: i32) -> bool {
    let len = current_list_len(state);
    if len == 0 {
        return false;
    }
    let old_selected = state.selected;
    let old_first_visible = state.first_visible;
    let new_selected = (state.selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
    state.selected = new_selected;
    if state.selected < state.first_visible {
        state.first_visible = state.selected;
    } else if state.selected >= state.first_visible + VISIBLE_ROWS {
        state.first_visible = state.selected - VISIBLE_ROWS + 1;
    }
    let moved = state.selected != old_selected || state.first_visible != old_first_visible;
    // Preview seulement si la sélection a bougé : sinon une flèche contre
    // une butée re-prévisualise le MÊME thème à chaque frappe
    // (preview_theme + recréation de police/pinceau) pour rien.
    if moved && state.mode == Mode::Theme {
        if let Some(&idx) = state.filtered.get(state.selected) {
            if let Some(name) = state.mode_items.get(idx).cloned() {
                theme::preview_theme(&mut state.theme, &name);
                // Les contrôles EDIT ont leur propre cycle de peinture
                // (voir apply_theme_visuals) : une preview en direct doit
                // les redessiner à CHAQUE flèche, sans quoi la barre de
                // recherche ne suit qu'à la prochaine frappe.
                apply_theme_visuals(state);
            }
        }
    }
    moved
}
