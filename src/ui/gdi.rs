//! Petites fonctions géométrie/police/classe de fenêtre partagées par les
//! trois fenêtres owner-drawn du projet (window.rs, popup_menu.rs,
//! tray.rs).

use crate::win32::gdi32::{
    CreateFontIndirectW, HFONT, LOGFONTW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    FF_DONTCARE, FW_NORMAL, OUT_DEFAULT_PRECIS,
};
use crate::win32::user32::{HCURSOR, WNDCLASSEXW, WNDPROC};
use crate::win32::{copy_into_wide, HINSTANCE, RECT};

pub fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

pub fn rect_w(r: &RECT) -> i32 {
    r.right - r.left
}

pub fn rect_h(r: &RECT) -> i32 {
    r.bottom - r.top
}

/// `lfQuality` est le seul champ dont la valeur change le rendu. Laissé à
/// `DEFAULT_QUALITY` (0), GDI choisit d'antialiaser ou non selon une
/// heuristique liée à la taille demandée : sur un écran non-4K, où la
/// fenêtre — et donc la police, dérivée de sa largeur — est physiquement
/// plus petite, le texte ressort crénelé. `CLEARTYPE_QUALITY` force
/// l'antialiasing sous-pixel à toute taille. Les autres champs valent déjà
/// 0 par défaut et ne sont explicités que par lisibilité.
pub fn make_font(family: &str, pixel_height: i32) -> HFONT {
    let mut lf = LOGFONTW {
        lfHeight: -pixel_height.max(1),
        lfWeight: FW_NORMAL as i32,
        lfCharSet: DEFAULT_CHARSET,
        lfOutPrecision: OUT_DEFAULT_PRECIS,
        lfClipPrecision: CLIP_DEFAULT_PRECIS,
        lfQuality: CLEARTYPE_QUALITY,
        lfPitchAndFamily: DEFAULT_PITCH | FF_DONTCARE,
        ..Default::default()
    };
    copy_into_wide(family, &mut lf.lfFaceName);
    unsafe { CreateFontIndirectW(&lf) }
}

/// `WNDCLASSEXW` avec les 8 champs identiques aux trois fenêtres
/// owner-drawn du projet ; seuls `class_name`, `wndproc`, `hinstance` et
/// `hcursor` varient de l'une à l'autre.
pub fn simple_wndclass(class_name: *const u16, wndproc: WNDPROC, hinstance: HINSTANCE, hcursor: HCURSOR) -> WNDCLASSEXW {
    WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: wndproc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: std::ptr::null_mut(),
        hCursor: hcursor,
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name,
        hIconSm: std::ptr::null_mut(),
    }
}
