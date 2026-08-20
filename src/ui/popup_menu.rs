//! Menu contextuel du tray, owner-drawn plutôt que le menu natif Win32
//! (TrackPopupMenu) : un menu natif ignore les couleurs de thème, et le
//! forcer en sombre exige des ordinaux non documentés de uxtheme.dll en plus
//! de SetWindowTheme -- fragile et dépendant de la version de Windows. Une
//! fenêtre à nous reste cohérente avec le reste de l'UI sans rien
//! d'officieux.
//!
//! Contrairement au reste de l'appli (une seule boucle de messages sur le
//! thread principal, voir main.rs), `show()` pompe sa propre boucle
//! imbriquée jusqu'à la fermeture du menu : un menu contextuel est une pause
//! modale de courte durée, et Win32 accepte une boucle `GetMessage`
//! imbriquée tant qu'elle ne passe jamais par `PostQuitMessage`, qui
//! enverrait aussi WM_QUIT à la boucle englobante.

use crate::win32::gdi32::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW,
    EndPaint, FillRect, GetDC, GetTextExtentPoint32W, GetTextMetricsW, InvalidateRect, ReleaseDC, SelectObject,
    SetBkMode, SetTextColor, BeginPaint, HBITMAP, HDC, HFONT, PAINTSTRUCT, SIZE, TEXTMETRICW, TRANSPARENT, DT_LEFT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, SRCCOPY,
};
use crate::win32::user32::{
    CreateWindowExW, DefWindowProcW, GetCursorPos, GetDpiForWindow, GetMessageW, GetWindowLongPtrW, LoadCursorW,
    RegisterClassExW, SetFocus, SetForegroundWindow, SetWindowLongPtrW, TrackMouseEvent, TranslateMessage,
    DispatchMessageW, DestroyWindow, GetSystemMetrics, GWLP_USERDATA, IDC_ARROW, TME_LEAVE, TRACKMOUSEEVENT,
    WA_INACTIVE, WM_ACTIVATE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONUP, WM_MOUSELEAVE, WM_MOUSEMOVE, WM_PAINT,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, HBRUSH,
};
use crate::win32::{to_wstring, HWND, LPARAM, LRESULT, MSG, POINT, RECT, UINT, WPARAM};
use super::gdi::{make_font, rect, rect_h, rect_w, simple_wndclass};

const WINDOW_CLASS_NAME: &str = "MAGILauncherMenuClass";
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;

pub enum MenuItem {
    Entry(String, String),
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
    /// Fond normal/survolé des lignes, créés une seule fois ici plutôt qu'un
    /// CreateSolidBrush/DeleteObject par item à chaque `WM_PAINT` (même
    /// approche que `AppState::list_bg_brush` dans window.rs).
    bg_brush: HBRUSH,
    selected_brush: HBRUSH,
    tracking: bool,
    /// Tampon hors-écran pour draw_menu, même rôle que `mem_dc` dans
    /// window.rs : draw_menu enchaîne plusieurs FillRect/DrawTextW (bordure,
    /// puis chaque item par-dessus), jamais une opération atomique. Dessiner
    /// directement sur le DC de la fenêtre laisse la DWM composer une frame
    /// intermédiaire au milieu de cette séquence, rejouée à chaque survol
    /// d'item (WM_MOUSEMOVE -> InvalidateRect -> WM_PAINT). Taille fixe pour
    /// toute la durée de vie de la fenêtre, calculée une fois dans `show()` :
    /// pas de garde-fou de redimensionnement à prévoir ici.
    mem_dc: HDC,
    mem_bitmap: HBITMAP,
    /// Taille du tampon -- constante pour toute la durée de vie de la
    /// fenêtre (voir la doc de mem_dc), gardée à part plutôt que
    /// recalculée à chaque WM_PAINT via GetClientRect.
    size: (i32, i32),
}

/// Redessine `state.mem_dc` en entier. À appeler après tout changement
/// visuel (survol) et avant `InvalidateRect`, pour que le prochain WM_PAINT
/// se réduise à un BitBlt.
unsafe fn redraw_into_buffer(state: &MenuState) {
    let (w, h) = state.size;
    let client = rect(0, 0, w, h);
    draw_menu(state.mem_dc, state, &client);
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut MenuState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MenuState;
    ptr.as_mut()
}

/// Récupère le DC d'écran, y sélectionne `font`, appelle `f`, puis nettoie.
/// `f` peut mesurer plusieurs chaînes sur ce même DC, au lieu d'un
/// GetDC/ReleaseDC par chaîne.
unsafe fn with_font_dc<T>(font: HFONT, f: impl FnOnce(HDC) -> T) -> T {
    let hdc = GetDC(std::ptr::null_mut());
    let old = SelectObject(hdc, font as _);
    let result = f(hdc);
    SelectObject(hdc, old);
    ReleaseDC(std::ptr::null_mut(), hdc);
    result
}

unsafe fn row_height_from_dc(hdc: HDC, pady: i32) -> i32 {
    let mut tm = TEXTMETRICW::default();
    GetTextMetricsW(hdc, &mut tm);
    tm.tmHeight + pady * 2
}

unsafe fn text_width_from_dc(hdc: HDC, text: &str) -> i32 {
    let wide = to_wstring(text);
    let mut size = SIZE::default();
    GetTextExtentPoint32W(hdc, wide.as_ptr(), (wide.len() as i32 - 1).max(0), &mut size);
    size.cx
}

fn y_from_lparam(lparam: LPARAM) -> i32 {
    (((lparam >> 16) & 0xFFFF) as u16) as i16 as i32
}

unsafe fn hit_test(state: &MenuState, y: i32) -> Option<usize> {
    state.item_rects.iter().position(|r| y >= r.top && y < r.bottom)
}

unsafe fn draw_menu(hdc: HDC, state: &MenuState, client: &RECT) {
    let brush = CreateSolidBrush(state.colors.border);
    FillRect(hdc, client, brush);
    DeleteObject(brush as _);

    SetBkMode(hdc, TRANSPARENT as i32);
    let old_font = SelectObject(hdc, state.font as _);

    for (i, item) in state.items.iter().enumerate() {
        let r = state.item_rects[i];
        let MenuItem::Entry(_, label) = item;
        let hovered = state.hover == Some(i);
        let (brush, fg) = if hovered {
            (state.selected_brush, state.colors.selected_text)
        } else {
            (state.bg_brush, state.colors.list_text)
        };
        FillRect(hdc, &r, brush);
        let pad = rect_h(&r) / 3;
        let mut text_rect = rect(r.left + pad, r.top, rect_w(&r) - 2 * pad, rect_h(&r));
        let wide = to_wstring(label);
        SetTextColor(hdc, fg);
        DrawTextW(hdc, wide.as_ptr(), (wide.len() as i32) - 1, &mut text_rect, DT_SINGLELINE | DT_VCENTER | DT_LEFT | DT_NOPREFIX);
    }

    SelectObject(hdc, old_font);
}

unsafe extern "system" fn menu_wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if let Some(state) = get_state(hwnd) {
                // Contenu déjà dessiné dans state.mem_dc par
                // redraw_into_buffer : présenté ici d'un seul BitBlt, jamais
                // draw_menu sur le DC réel (voir la doc de mem_dc).
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let (w, h) = state.size;
                BitBlt(hdc, 0, 0, w, h, state.mem_dc, 0, 0, SRCCOPY);
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
                    redraw_into_buffer(state);
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
                    redraw_into_buffer(state);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            if let Some(state) = get_state(hwnd) {
                if let Some(i) = state.hover {
                    let MenuItem::Entry(id, _) = &state.items[i];
                    state.selected = Some(id.clone());
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
                if !state.bg_brush.is_null() {
                    DeleteObject(state.bg_brush as _);
                }
                if !state.selected_brush.is_null() {
                    DeleteObject(state.selected_brush as _);
                }
                if !state.mem_bitmap.is_null() {
                    DeleteObject(state.mem_bitmap as _);
                }
                if !state.mem_dc.is_null() {
                    DeleteDC(state.mem_dc);
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
    // Échoue si la classe est déjà enregistrée (menu rouvert) : sans
    // conséquence, retour ignoré volontairement.
    RegisterClassExW(&wc);

    // La fenêtre principale dérive sa taille de police de sa largeur, déjà
    // exprimée en pixels physiques du moniteur. Ce menu n'a aucune dimension
    // de référence à dériver : il lui faut le ratio DPI explicite pour
    // choisir une taille physiquement correcte.
    let dpi = GetDpiForWindow(parent).max(1);
    let scale = dpi as f64 / 96.0;
    let font = make_font(font_family, (11.0 * scale).round() as i32);
    let padx = (14.0 * scale).round() as i32;
    let pady = (6.0 * scale).round() as i32;

    // Un seul DC pour la hauteur de ligne et tous les libellés.
    let (row_h, width) = with_font_dc(font, |hdc| {
        let row_h = row_height_from_dc(hdc, pady);
        let width = items
            .iter()
            .map(|MenuItem::Entry(_, label)| text_width_from_dc(hdc, label) + padx * 2)
            .max()
            .unwrap_or(0);
        (row_h, width)
    });

    let mut item_rects = Vec::with_capacity(items.len());
    let mut y = border_width;
    for _ in &items {
        item_rects.push(rect(border_width, y, width, row_h));
        y += row_h;
    }
    let content_h = y - border_width;
    let window_w = width + border_width * 2;
    let window_h = content_h + border_width * 2;

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

    let bg_brush = CreateSolidBrush(colors.list_background);
    let selected_brush = CreateSolidBrush(colors.selected_background);

    // Tampon hors-écran (voir la doc de mem_dc dans MenuState), de taille
    // fixe, créé une seule fois ici.
    let screen_dc = GetDC(std::ptr::null_mut());
    let mem_dc = CreateCompatibleDC(screen_dc);
    let mem_bitmap = CreateCompatibleBitmap(screen_dc, window_w.max(1), window_h.max(1));
    SelectObject(mem_dc, mem_bitmap as _);
    ReleaseDC(std::ptr::null_mut(), screen_dc);

    let mut state = Box::new(MenuState {
        items,
        hover: None,
        selected: None,
        item_rects,
        colors,
        font,
        bg_brush,
        selected_brush,
        tracking: false,
        mem_dc,
        mem_bitmap,
        size: (window_w, window_h),
    });
    // Premier dessin avant ShowWindow : le WM_PAINT initial se réduit à un
    // BitBlt et n'affiche jamais de contenu vide.
    redraw_into_buffer(&state);
    let state_ptr = &mut *state as *mut MenuState;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

    crate::win32::user32::ShowWindow(hwnd, crate::win32::user32::SW_SHOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);

    // Boucle imbriquée : la sortie se fait en surveillant la destruction de
    // CETTE fenêtre, jamais via PostQuitMessage/WM_QUIT (voir l'en-tête).
    let mut msg = MSG::default();
    while crate::win32::user32::IsWindow(hwnd) != 0 && GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    state.selected.take()
}
