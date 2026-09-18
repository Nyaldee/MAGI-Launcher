//! Rendu GDI de la scène (bordure, recherche/horloge, séparateur, lignes de
//! résultat) et du placeholder de l'EDIT natif.

use crate::win32::gdi32::{
    CreateSolidBrush, DeleteObject, DrawTextW, FillRect, SelectObject, SetBkMode, SetTextColor, HDC, TRANSPARENT,
    DT_END_ELLIPSIS, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
};
use crate::win32::user32::GetClientRect;
use crate::win32::{to_wstring, HWND, RECT};

use crate::ui::gdi::{rect, rect_h, rect_w};

use super::app_state::{AppState, SearchDisplay};
use super::geometry::{full_window_rect, text_margin_px};
use super::items::row_label;

/// `true` si `a` et `b` ont une intersection non vide -- permet à
/// draw_scene de sauter un bloc entièrement hors de la zone invalidée (voir
/// invalidate_after_navigation, qui n'invalide parfois qu'une ou deux
/// lignes). Le rect vient de `ps.rcPaint`, jamais recalculé nous-mêmes.
fn rects_intersect(a: &RECT, b: &RECT) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// `dirty` -- la zone à réellement redessiner (`ps.rcPaint` de l'appelant).
/// Chaque bloc (bordure, recherche/horloge, séparateur, chaque ligne) est
/// sauté s'il n'intersecte pas `dirty`, pour qu'une invalidation partielle
/// (voir invalidate_after_navigation) ne coûte que les lignes concernées.
/// Sans ce filtrage, WM_PAINT redessine toute la scène même quand Windows
/// ne demande qu'un petit rect -- surcoût net à window_size élevé, où
/// chaque frame rastérise beaucoup plus de pixels et de glyphes.
pub(crate) unsafe fn draw_scene(hdc: HDC, state: &AppState, dirty: &RECT) {
    let g = &state.geometry;
    let t = &state.theme.current;

    // Fond de bordure sur toute la fenêtre, puis la barre de recherche et
    // le séparateur par-dessus (rectangles imbriqués) -- la bordure n'est
    // jamais qu'une couleur de fond qui dépasse.
    let full = full_window_rect(g);
    if rects_intersect(&full, dirty) {
        FillRect(hdc, &full, state.border_brush);
    }
    // Recherche et horloge forment UN SEUL bloc visuel search_background
    // (voir compute_geometry) : les deux rects doivent être remplis. Sans
    // celui de g.clock, les zones du bloc horloge que le texte ne couvre
    // pas laissent voir la couleur de bordure du fill plein-fenêtre.
    if rects_intersect(&g.search, dirty) {
        FillRect(hdc, &g.search, state.search_brush);
    }
    if rects_intersect(&g.clock, dirty) {
        FillRect(hdc, &g.clock, state.search_brush);
    }
    if rects_intersect(&g.separator, dirty) {
        FillRect(hdc, &g.separator, state.border_brush);
    }

    SetBkMode(hdc, TRANSPARENT as i32);

    // Horloge dessinée ici, dans le même tampon hors-écran que le reste
    // (voir mem_dc dans AppState), et non via un contrôle EDIT séparé : un
    // contrôle réel a son propre cycle de peinture, déclenché chaque
    // seconde par CLOCK_TIMER_ID et composé par la DWM hors de l'atomicité
    // que ce tampon garantit -- exactement le clignotement corrigé pour le
    // corps de la liste. `font_search` est sélectionnée et relâchée ici
    // pour ne pas affecter le reste de la fonction.
    if state.theme.show_clock && rects_intersect(&g.clock, dirty) {
        let clock_font = SelectObject(hdc, state.font_search as _);
        draw_clock_text(hdc, &g.clock, &crate::core::clock::format_now(), t.search_text, state.text_margin);
        SelectObject(hdc, clock_font);
    }

    let old_font = SelectObject(hdc, state.font_row as _);

    match &state.display {
        SearchDisplay::Color(color) => {
            for row in g.rows.iter() {
                if rects_intersect(row, dirty) {
                    fill_color(hdc, row, *color);
                }
            }
        }
        SearchDisplay::Calc(text) | SearchDisplay::SingleLine(text) => {
            if rects_intersect(&g.rows[0], dirty) {
                FillRect(hdc, &g.rows[0], state.selected_bg_brush);
                draw_row_text(hdc, &g.rows[0], text, t.selected_text);
            }
            for row in g.rows[1..].iter() {
                if rects_intersect(row, dirty) {
                    FillRect(hdc, row, state.list_bg_brush);
                }
            }
        }
        SearchDisplay::List => {
            for (slot, row) in g.rows.iter().enumerate() {
                if !rects_intersect(row, dirty) {
                    continue;
                }
                let list_index = state.first_visible + slot;
                match state.filtered.get(list_index) {
                    Some(&item_index) => {
                        let selected = list_index == state.selected;
                        let (brush, fg) = if selected {
                            (state.selected_bg_brush, t.selected_text)
                        } else {
                            (state.list_bg_brush, t.list_text)
                        };
                        FillRect(hdc, row, brush);
                        draw_row_text(hdc, row, &row_label(state, item_index), fg);
                    }
                    None => {
                        FillRect(hdc, row, state.list_bg_brush);
                    }
                }
            }
        }
    }

    SelectObject(hdc, old_font);
}

/// Seul cas où la couleur n'est PAS un des pinceaux de thème mis en cache
/// (voir AppState) : l'aperçu de couleur hexadécimale tapée dans la
/// recherche, arbitraire et jamais connue à l'avance.
unsafe fn fill_color(hdc: HDC, r: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, r, brush);
    DeleteObject(brush as _);
}

unsafe fn draw_row_text(hdc: HDC, row: &RECT, text: &str, color: u32) {
    let pad = text_margin_px(rect_h(row));
    let mut text_rect = rect(row.left + pad, row.top, rect_w(row) - 2 * pad, rect_h(row));
    // Retours à la ligne (notes collées, cibles...) aplatis en espaces :
    // une ligne de la liste a une hauteur fixe, jamais prévue pour du
    // multi-ligne. `replace` alloue toujours une String, d'où le Cow --
    // cette fonction tourne pour chaque ligne à chaque repaint, et le cas
    // de loin le plus fréquent ne contient ni \n ni \r.
    let flattened: std::borrow::Cow<str> =
        if text.contains(['\n', '\r']) { text.replace(['\n', '\r'], " ").into() } else { text.into() };
    let wide = to_wstring(&flattened);
    SetTextColor(hdc, color);
    DrawTextW(
        hdc,
        wide.as_ptr(),
        (wide.len() as i32) - 1, // sans le NUL terminal
        &mut text_rect,
        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}

/// Dessine l'heure alignée à droite dans `block`, le bloc visuel complet et
/// non un sous-rect pré-centré (DT_VCENTER s'en charge, comme dans
/// draw_row_text). `margin` vaut toujours `state.text_margin` : la même
/// marge que tout autre texte, pour un alignement identique à celui de la
/// barre de recherche voisine.
unsafe fn draw_clock_text(hdc: HDC, block: &RECT, text: &str, color: u32, margin: i32) {
    let mut text_rect = rect(block.left + margin, block.top, rect_w(block) - 2 * margin, rect_h(block));
    let wide = to_wstring(text);
    SetTextColor(hdc, color);
    DrawTextW(hdc, wide.as_ptr(), (wide.len() as i32) - 1, &mut text_rect, DT_SINGLELINE | DT_VCENTER | DT_RIGHT | DT_NOPREFIX);
}

/// Dessine le texte d'invite à la main quand le champ est vide, avec la
/// police et la couleur du thème courant -- remplace EM_SETCUEBANNER, qui
/// peint avec une couleur interne à comctl32 ignorant SetTextColor et ne
/// suit donc jamais un changement de thème. `hdc` vient de l'appelant
/// plutôt que d'un GetDC local : le placeholder doit rester dans la même
/// session BeginPaint/EndPaint que le fond (voir edit_subclass_proc).
pub(crate) unsafe fn draw_placeholder(hdc: HDC, hwnd: HWND, state: &AppState) {
    let mut rc = RECT::default();
    GetClientRect(hwnd, &mut rc);
    rc.left += state.text_margin;
    let old_font = SelectObject(hdc, state.font_search as _);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, state.theme.current.search_text);
    let len = (state.placeholder_wide.len() as i32 - 1).max(0);
    DrawTextW(hdc, state.placeholder_wide.as_ptr(), len, &mut rc, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
    SelectObject(hdc, old_font);
}
