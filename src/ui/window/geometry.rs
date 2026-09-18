//! Calcul de géométrie pur (aucun état de fenêtre touché) + son application
//! réelle à la fenêtre et ses contrôles enfants.
//!
//! Largeur = fraction de l'écran (themes.json), hauteur = largeur * 9/16
//! strictement, contenu divisé en tranches égales (2 pour la barre de
//! recherche + 10 pour les résultats) -- entièrement dérivé de la taille de
//! fenêtre, jamais de dimensions fixes.

use crate::win32::gdi32::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO};
use crate::win32::user32::{GetCursorPos, SetWindowPos, HWND_TOPMOST, MONITOR_DEFAULTTONEAREST, SWP_NOZORDER};
use crate::win32::{HWND, POINT, RECT};

use crate::ui::gdi::{rect, rect_h, rect_w};
use crate::ui::theme::ThemeConfig;

use super::app_state::{apply_theme_visuals, AppState};
use super::force_repaint_now;

pub const VISIBLE_ROWS: usize = 10;
/// Nombre de "tranches" égales du contenu : 2 pour la barre de recherche
/// (volontairement deux fois plus grande qu'une ligne de résultat) +
/// VISIBLE_ROWS pour les résultats -- toute la géométrie en découle.
const CONTENT_UNITS: i32 = VISIBLE_ROWS as i32 + 2;
/// Fraction de la largeur de la barre de recherche réservée à l'horloge
/// quand `show_clock` est actif.
const CLOCK_WIDTH_FRACTION: f64 = 0.22;
/// Ratio hauteur/largeur de la fenêtre -- 16:9 strict, indépendant du
/// contenu.
const HEIGHT_RATIO: f64 = 9.0 / 16.0;

#[derive(Clone, Copy, Default)]
pub(crate) struct Geometry {
    pub(crate) window: RECT,
    /// Rects ci-dessous en coordonnées CLIENTES (0,0 = coin haut-gauche de
    /// la fenêtre), un seul jeu de calculs partagé par le repositionnement
    /// des contrôles enfants et par le rendu GDI.
    pub(crate) search: RECT,
    /// Vide (largeur nulle) quand `show_clock` est désactivé.
    pub(crate) clock: RECT,
    pub(crate) separator: RECT,
    pub(crate) rows: [RECT; VISIBLE_ROWS],
}

/// Marge générale unique que TOUT texte respecte (lignes de résultat,
/// placeholder, horloge) -- tout le reste (blocs, rects, contrôles) va bord
/// à bord ; seule cette marge insère du vide avant le texte. Point de
/// calcul unique : un ratio dupliqué ailleurs divergerait au premier
/// arrondi.
pub(crate) fn text_margin_px(row_h: i32) -> i32 {
    (row_h as f64 * 0.3) as i32
}

/// Rect du contrôle natif à l'intérieur de son bloc visuel -- le bloc fait
/// deux fois la hauteur d'une ligne (voir compute_geometry), le contrôle
/// garde `control_h` et est centré dedans. Un EDIT single-line étiré à une
/// hauteur très disproportionnée par rapport à sa police ne centre plus
/// fiablement son caret par rapport au texte.
fn centered_control_rect(block: &RECT, control_h: i32) -> RECT {
    let top = block.top + (rect_h(block) - control_h) / 2;
    rect(block.left, top, rect_w(block), control_h)
}

/// Rect du contrôle EDIT de recherche -- calcul partagé par `create`
/// (position initiale) et `apply_geometry` (repositionnement au
/// Reload/changement de moniteur). L'horloge n'a pas de contrôle réel
/// (voir draw_clock_text) : `geometry.clock` sert directement au dessin.
pub(crate) fn search_control_rect(geometry: &Geometry) -> RECT {
    let control_h = rect_h(&geometry.rows[0]);
    centered_control_rect(&geometry.search, control_h)
}

/// Zone de travail (écran moins barre des tâches) du moniteur SOUS LE
/// CURSEUR -- pas forcément le moniteur principal, comportement multi-
/// écran façon Rofi : la popup s'ouvre toujours là où est la souris.
pub(crate) fn work_area_under_cursor() -> RECT {
    unsafe {
        let mut pt = POINT::default();
        GetCursorPos(&mut pt);
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            info.rcWork
        } else {
            RECT { left: 0, top: 0, right: 1920, bottom: 1080 }
        }
    }
}

pub(crate) fn compute_geometry(work: RECT, theme: &ThemeConfig) -> Geometry {
    let window_w = (rect_w(&work) as f64 * theme.window_width_fraction).round() as i32;
    let window_h = (window_w as f64 * HEIGHT_RATIO).round() as i32;
    // Centrée sur les deux axes. Le .min(...).max(...) reste nécessaire :
    // si window_w/h dépasse `work` (cas extrême), on cale contre le bord
    // haut/gauche plutôt que de laisser une valeur négative pousser la
    // fenêtre hors écran de l'autre côté.
    let window_x = (work.left + (rect_w(&work) - window_w) / 2).min(work.right - window_w).max(work.left);
    let window_y = (work.top + (rect_h(&work) - window_h) / 2).min(work.bottom - window_h).max(work.top);

    // 0 est une valeur légitime (popup sans bordure ni séparateur), d'où
    // .max(0) et pas de plancher à 1 -- unit_h a son propre plancher,
    // indépendant de border.
    let border = theme.border_width.max(0);
    let content_w = (window_w - 2 * border).max(0);
    let content_h = (window_h - 2 * border).max(0);
    let separator_h = border;
    let unit_h = ((content_h - separator_h) / CONTENT_UNITS).max(1);
    let search_h = unit_h * 2;

    let (search, clock) = if theme.show_clock {
        let clock_w = (content_w as f64 * CLOCK_WIDTH_FRACTION).round() as i32;
        let search_w = (content_w - clock_w).max(0);
        (rect(border, border, search_w, search_h), rect(border + search_w, border, clock_w, search_h))
    } else {
        (rect(border, border, content_w, search_h), RECT::default())
    };
    let separator = rect(border, border + search_h, content_w, separator_h);
    // La division entière de unit_h laisse un reste de quelques pixels.
    // Non absorbé, il s'ajoute à la bordure du bas (jamais couverte par une
    // ligne), qui paraît alors plus épaisse que les trois autres côtés --
    // la dernière ligne le prend donc à sa charge.
    let remainder = (content_h - separator_h) - unit_h * CONTENT_UNITS;
    let mut rows = [RECT::default(); VISIBLE_ROWS];
    let mut y = separator.bottom;
    for (i, row) in rows.iter_mut().enumerate() {
        let h = if i == VISIBLE_ROWS - 1 { unit_h + remainder } else { unit_h };
        *row = rect(border, y, content_w, h);
        y += h;
    }

    Geometry { window: rect(window_x, window_y, window_w, window_h), search, clock, separator, rows }
}

/// Rectangle de la fenêtre entière, en coordonnées clientes -- basé sur les
/// dimensions réelles (`g.window`) et non reconstruit à partir des rects de
/// lignes : les arrondis de la division entière de compute_geometry
/// sous-estimeraient de quelques pixels, laissant un liseré non repeint en
/// bas de fenêtre.
pub(crate) fn full_window_rect(g: &Geometry) -> RECT {
    rect(0, 0, rect_w(&g.window), rect_h(&g.window))
}

/// Rect englobant les VISIBLE_ROWS lignes -- toutes partagent le même
/// left/width (voir compute_geometry), seul leur Y diffère : du haut de la
/// première au bas de la dernière les couvre exactement, sans la bordure ni
/// la barre de recherche au-dessus.
pub(crate) fn rows_band_rect(g: &Geometry) -> RECT {
    let first = g.rows[0];
    let last = g.rows[VISIBLE_ROWS - 1];
    rect(first.left, first.top, rect_w(&first), last.bottom - first.top)
}

/// Recalcule la géométrie (moniteur sous le curseur + réglages de thème
/// courants) et repositionne/redimensionne la fenêtre ET ses contrôles
/// enfants -- partagé par show() (le moniteur sous le curseur a pu changer)
/// et reload_config() (largeur/bordure ont pu changer dans themes.json).
/// `state.geometry` doit rester corrélée à la taille réelle à l'écran :
/// recalculer sans appliquer les désynchronise.
///
/// L'horloge n'a pas de contrôle à repositionner (voir draw_clock_text) --
/// activer/désactiver `show_clock` prend donc effet dès le Reload, sans
/// redémarrage.
pub(crate) unsafe fn apply_geometry(hwnd: HWND, state: &mut AppState) {
    let work = work_area_under_cursor();
    state.geometry = compute_geometry(work, &state.theme);
    apply_theme_visuals(state);
    let g = state.geometry.window;
    let edit_rect = search_control_rect(&state.geometry);
    SetWindowPos(hwnd, HWND_TOPMOST, g.left, g.top, rect_w(&g), rect_h(&g), SWP_NOZORDER);
    SetWindowPos(
        state.edit_hwnd,
        std::ptr::null_mut(),
        edit_rect.left,
        edit_rect.top,
        rect_w(&edit_rect),
        rect_h(&edit_rect),
        SWP_NOZORDER,
    );
}

/// Ctrl+1..9/0 : bascule window_size sur `percent` (10..100) et le persiste
/// aussitôt dans state.json, même commit immédiat que le sélecteur de
/// thème. L'écriture disque est best-effort (state.json en lecture seule,
/// par exemple) : l'affichage a déjà changé, un échec de persistance n'a
/// pas à faire échouer l'action visible.
pub(crate) unsafe fn set_window_size_percent(hwnd: HWND, state: &mut AppState, percent: i32) {
    let new_fraction = (percent as f64 / 100.0).clamp(0.05, 1.0);
    // No-op si la taille est déjà celle demandée : sans ce garde-fou,
    // maintenir Ctrl+1 relance à chaque frappe tout le cycle (recalcul de
    // géométrie, SetWindowPos, invalidation plein écran, lecture + écriture
    // de themes.json) pour un résultat identique.
    if new_fraction == state.theme.window_width_fraction {
        return;
    }
    state.theme.window_width_fraction = new_fraction;
    apply_geometry(hwnd, state);
    force_repaint_now(hwnd);
    let _ = crate::core::state::commit_window_size(&state.state_path, percent);
}

/// Ctrl+-/Ctrl+= (voir handle_edit_keydown) : ajuste l'épaisseur de bordure
/// de `delta` px et la persiste tout de suite, même principe que
/// set_window_size_percent ci-dessus -- y compris le même garde-fou contre
/// un no-op (bordure déjà à 0 ou déjà à sa borne haute).
pub(crate) unsafe fn adjust_border(hwnd: HWND, state: &mut AppState, delta: i32) {
    let new_border = (state.theme.border_width + delta).clamp(0, 100);
    if new_border == state.theme.border_width {
        return;
    }
    state.theme.border_width = new_border;
    apply_geometry(hwnd, state);
    force_repaint_now(hwnd);
    let _ = crate::core::state::commit_border(&state.state_path, state.theme.border_width);
}
