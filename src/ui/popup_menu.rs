//! Menu contextuel du tray, owner-drawn -- pas le menu natif Win32
//! (TrackPopupMenu). Un menu natif ignore les couleurs de thème ; le faire
//! rendre en sombre demande des API non documentées (SetWindowTheme seul
//! ne suffit pas, il faut aussi des ordinaux non documentés de
//! uxtheme.dll pour forcer le mode sombre du processus) -- fragile et
//! dépendant de la version de Windows. Une fenêtre à nous donne un
//! contrôle total et cohérent avec le reste de l'UI, sans rien
//! d'officieux.
//!
//! Contrairement au reste de l'appli (une seule boucle de messages sur le
//! thread principal, voir main.rs), `show()` pompe sa PROPRE petite boucle
//! de messages, imbriquée, jusqu'à ce que le menu se ferme -- un menu
//! contextuel est par nature une pause modale de courte durée, et Win32
//! autorise sans problème une boucle `GetMessage` imbriquée tant qu'elle
//! ne s'appuie jamais sur `PostQuitMessage` (ça enverrait aussi WM_QUIT à
//! la boucle englobante).

use crate::win32::gdi32::{
    CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetTextExtentPoint32W, GetTextMetricsW,
    InvalidateRect, ReleaseDC, SelectObject, SetBkMode, SetTextColor, BeginPaint, HDC, HFONT, PAINTSTRUCT, SIZE,
    TEXTMETRICW, TRANSPARENT, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
};
use crate::win32::user32::{
    CreateWindowExW, DefWindowProcW, GetCursorPos, GetDpiForWindow, GetMessageW, GetWindowLongPtrW, LoadCursorW,
    RegisterClassExW, SetFocus, SetForegroundWindow, SetWindowLongPtrW, TrackMouseEvent, TranslateMessage,
    DispatchMessageW, DestroyWindow, GetSystemMetrics, GWLP_USERDATA, IDC_ARROW, TME_LEAVE, TRACKMOUSEEVENT,
    WA_INACTIVE, WM_ACTIVATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use crate::win32::{to_wstring, HWND, LPARAM, LRESULT, MSG, POINT, RECT, UINT, WPARAM};
use super::gdi::{make_font, rect, rect_h, rect_w, simple_wndclass};

const WINDOW_CLASS_NAME: &str = "MAGILauncherMenuClass";
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;

pub enum MenuItem {
    Entry(String, String),
    /// Le menu du tray actuel (main.rs) n'en a pas besoin, mais le rendu
    /// (draw_menu) et le calcul de géométrie la gèrent déjà -- gardée pour
    /// un futur menu qui en aurait l'usage plutôt que retirée puis
    /// réécrite le jour venu.
    #[allow(dead_code)]
    Separator,
}

pub struct MenuColors {
    pub list_background: u32,
    pub list_text: u32,
    pub selected_background: u32,
    pub selected_text: u32,
    pub border: u32,
}

struct MenuState {
    items: Vec<MenuItem>,
    hover: Option<usize>,
    selected: Option<String>,
    /// Rects de chaque item en coordonnées clientes -- précalculés une
    /// fois à la création, jamais recalculés ensuite (taille fixe).
    item_rects: Vec<RECT>,
    colors: MenuColors,
    font: HFONT,
    tracking: bool,
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut MenuState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MenuState;
    ptr.as_mut()
}

unsafe fn measure_row_height(font: HFONT, pady: i32) -> i32 {
    let hdc = GetDC(std::ptr::null_mut());
    let old = SelectObject(hdc, font as _);
    let mut tm = TEXTMETRICW::default();
    GetTextMetricsW(hdc, &mut tm);
    SelectObject(hdc, old);
    ReleaseDC(std::ptr::null_mut(), hdc);
    tm.tmHeight + pady * 2
}

unsafe fn measure_text_width(font: HFONT, text: &str) -> i32 {
    let hdc = GetDC(std::ptr::null_mut());
    let old = SelectObject(hdc, font as _);
    let wide = to_wstring(text);
    let mut size = SIZE::default();
    GetTextExtentPoint32W(hdc, wide.as_ptr(), (wide.len() as i32 - 1).max(0), &mut size);
    SelectObject(hdc, old);
    ReleaseDC(std::ptr::null_mut(), hdc);
    size.cx
}

fn y_from_lparam(lparam: LPARAM) -> i32 {
    (((lparam >> 16) & 0xFFFF) as u16) as i16 as i32
}

unsafe fn hit_test(state: &MenuState, y: i32) -> Option<usize> {
    state.item_rects.iter().position(|r| y >= r.top && y < r.bottom).filter(|&i| matches!(state.items[i], MenuItem::Entry(..)))
}

unsafe fn draw_menu(hdc: HDC, state: &MenuState, client: &RECT) {
    let brush = CreateSolidBrush(state.colors.border);
    FillRect(hdc, client, brush);
    DeleteObject(brush as _);

    SetBkMode(hdc, TRANSPARENT as i32);
    let old_font = SelectObject(hdc, state.font as _);

    for (i, item) in state.items.iter().enumerate() {
        let r = state.item_rects[i];
        match item {
            MenuItem::Separator => {
                let b = CreateSolidBrush(state.colors.border);
                FillRect(hdc, &r, b);
                DeleteObject(b as _);
            }
            MenuItem::Entry(_, label) => {
                let hovered = state.hover == Some(i);
                let (bg, fg) = if hovered {
                    (state.colors.selected_background, state.colors.selected_text)
                } else {
                    (state.colors.list_background, state.colors.list_text)
                };
                let brush = CreateSolidBrush(bg);
                FillRect(hdc, &r, brush);
                DeleteObject(brush as _);
                let pad = rect_h(&r) / 3;
                let mut text_rect = rect(r.left + pad, r.top, rect_w(&r) - 2 * pad, rect_h(&r));
                let wide = to_wstring(label);
                SetTextColor(hdc, fg);
                DrawTextW(
                    hdc,
                    wide.as_ptr(),
                    (wide.len() as i32) - 1,
                    &mut text_rect,
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_NOPREFIX,
                );
            }
        }
    }

    SelectObject(hdc, old_font);
}

unsafe extern "system" fn menu_wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if let Some(state) = get_state(hwnd) {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let client = rect(0, 0, ps.rcPaint.right, ps.rcPaint.bottom);
                draw_menu(hdc, state, &client);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Some(state) = get_state(hwnd) {
                if !state.tracking {
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut tme);
                    state.tracking = true;
                }
                let y = y_from_lparam(lparam);
                let new_hover = hit_test(state, y);
                if new_hover != state.hover {
                    state.hover = new_hover;
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_MOUSELEAVE => {
            if let Some(state) = get_state(hwnd) {
                state.tracking = false;
                if state.hover.is_some() {
                    state.hover = None;
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            if let Some(state) = get_state(hwnd) {
                if let Some(i) = state.hover {
                    if let MenuItem::Entry(id, _) = &state.items[i] {
                        state.selected = Some(id.clone());
                    }
                }
            }
            DestroyWindow(hwnd);
            0
        }
        WM_ACTIVATE => {
            if (wparam & 0xFFFF) as u32 == WA_INACTIVE {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MenuState;
            if !ptr.is_null() {
                let state = &*ptr;
                if !state.font.is_null() {
                    DeleteObject(state.font as _);
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Affiche le menu à la position du curseur et bloque (boucle de messages
/// imbriquée, voir le commentaire en tête de fichier) jusqu'à sa
/// fermeture. Retourne l'id de l'item cliqué, ou `None` si fermé sans
/// sélection (clic en dehors, perte de focus...).
pub unsafe fn show(parent: HWND, colors: MenuColors, font_family: &str, border_width: i32, items: Vec<MenuItem>) -> Option<String> {
    let class_name = to_wstring(WINDOW_CLASS_NAME);
    let wc = simple_wndclass(
        class_name.as_ptr(),
        Some(menu_wndproc),
        std::ptr::null_mut(),
        LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
    );
    // Un ré-enregistrement échoue silencieusement (classe déjà connue) --
    // sans conséquence, on ignore volontairement le retour ici.
    RegisterClassExW(&wc);

    // dpi_scale à la Tk (winfo_fpixels('1i')/96) : contrairement à la
    // fenêtre principale (dont la taille de police découle de la largeur
    // de fenêtre, elle-même déjà en pixels physiques du moniteur), ce menu
    // n'a pas de dimension de référence à dériver -- il lui faut le ratio
    // DPI explicite pour choisir une taille de police physiquement
    // correcte, exactement le rôle que jouait ce même calcul côté Tk.
    let dpi = GetDpiForWindow(parent).max(1);
    let scale = dpi as f64 / 96.0;
    let font = make_font(font_family, (11.0 * scale).round() as i32);
    let padx = (14.0 * scale).round() as i32;
    let pady = (6.0 * scale).round() as i32;
    let sep_margin = (4.0 * scale).round() as i32;

    let row_h = measure_row_height(font, pady);
    let sep_h = 1 + sep_margin * 2;

    let mut width = 0;
    let mut item_rects = Vec::with_capacity(items.len());
    let mut y = border_width;
    for item in &items {
        match item {
            MenuItem::Separator => {
                item_rects.push(rect(border_width, y, 0, sep_h));
                y += sep_h;
            }
            MenuItem::Entry(_, label) => {
                width = width.max(measure_text_width(font, label) + padx * 2);
                item_rects.push(rect(border_width, y, 0, row_h));
                y += row_h;
            }
        }
    }
    let content_h = y - border_width;
    let window_w = width + border_width * 2;
    let window_h = content_h + border_width * 2;
    for r in item_rects.iter_mut() {
        r.right = r.left + width;
    }

    let mut cursor = POINT::default();
    GetCursorPos(&mut cursor);
    let screen_w = GetSystemMetrics(SM_CXSCREEN);
    let screen_h = GetSystemMetrics(SM_CYSCREEN);
    let x = cursor.x.min(screen_w - window_w).max(0);
    let y_pos = cursor.y.min(screen_h - window_h).max(0);

    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        class_name.as_ptr(),
        std::ptr::null(),
        WS_POPUP,
        x,
        y_pos,
        window_w,
        window_h,
        parent,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        DeleteObject(font as _);
        return None;
    }

    let mut state = Box::new(MenuState {
        items,
        hover: None,
        selected: None,
        item_rects,
        colors,
        font,
        tracking: false,
    });
    let state_ptr = &mut *state as *mut MenuState;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

    crate::win32::user32::ShowWindow(hwnd, crate::win32::user32::SW_SHOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);

    // Boucle de messages imbriquée -- voir le commentaire en tête de
    // fichier sur pourquoi ni PostQuitMessage ni WM_QUIT n'interviennent
    // ici : la sortie se fait uniquement en surveillant la destruction de
    // CETTE fenêtre.
    let mut msg = MSG::default();
    while crate::win32::user32::IsWindow(hwnd) != 0 && GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    state.selected.take()
}
