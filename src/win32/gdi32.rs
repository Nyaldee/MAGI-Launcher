//! Primitives de dessin (device contexts, polices, brosses) + les API de
//! moniteur/peinture que windows-sys regroupe sous son module `Gdi` même
//! si certaines (BeginPaint, GetDC, MonitorFromPoint...) sont en réalité
//! exportées par user32.dll sur le vrai Windows -- l'organisation de
//! windows-sys est thématique, pas par DLL ; le lien vers la bonne DLL est
//! déjà correct à l'intérieur de chaque déclaration, seul l'endroit où
//! vivent les *types Rust* correspondants change.

pub use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontIndirectW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetMonitorInfoW,
    GetStockObject, GetTextExtentPoint32W, GetTextMetricsW, InvalidateRect, MonitorFromPoint, ReleaseDC,
    SelectObject, SetBkColor, SetBkMode, SetTextColor, TextOutW, UpdateWindow, HBITMAP, HDC, HFONT, HGDIOBJ, HPEN, LOGFONTW,
    MONITORINFO, PAINTSTRUCT, TEXTMETRICW,
    // constantes
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_BOLD, FW_NORMAL, OPAQUE, OUT_DEFAULT_PRECIS, SRCCOPY,
    TRANSPARENT,
};
// SIZE est une struct générique côté Foundation, pas spécifique à Gdi --
// windows-sys l'y range malgré son usage ici avec GetTextExtentPoint32W.
pub use windows_sys::Win32::Foundation::SIZE;

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}
