//! Petites fonctions géométrie/police/classe de fenêtre partagées par les
//! trois fenêtres owner-drawn du projet (window.rs, popup_menu.rs,
//! tray.rs).

use crate::win32::gdi32::{CreateFontIndirectW, HFONT, LOGFONTW};
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

pub fn make_font(family: &str, pixel_height: i32) -> HFONT {
    let mut lf = LOGFONTW { lfHeight: -pixel_height.max(1), ..Default::default() };
    copy_into_wide(family, &mut lf.lfFaceName);
    unsafe { CreateFontIndirectW(&lf) }
}

/// `WNDCLASSEXW` "simple" : les 8 champs qui ne varient jamais entre les
/// trois fenêtres owner-drawn du projet (popup principal/rebond DVD, tray
/// invisible, menu contextuel) -- seuls `wndproc`/`hinstance`/`hcursor`/
/// `class_name` changent d'une fenêtre à l'autre.
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
