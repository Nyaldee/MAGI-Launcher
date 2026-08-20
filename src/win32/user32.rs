//! user32.dll -- fenêtres, messages, entrée, hotkeys, moniteurs, DPI.
//! Ré-exporté depuis windows-sys ; les types de handles (HICON, HCURSOR,
//! HMENU) viennent du même module `WindowsAndMessaging` où windows-sys les
//! définit.

pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
    DispatchMessageW, EnumWindows, FindWindowW, GetCaretPos, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetMessageW, GetParent, GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowLongW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, HideCaret, IsIconic, IsWindow,
    IsWindowVisible, KillTimer, LoadCursorW, LoadIconW, LoadImageW, PeekMessageW, PostMessageW, PostQuitMessage,
    RegisterClassExW, SendMessageW, SetCaretPos, SetCursor, SetForegroundWindow, SetProcessDPIAware, SetTimer,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowCaret, ShowWindow, TranslateMessage, CREATESTRUCTW,
    HCURSOR, HICON, HMENU, WNDCLASSEXW, WNDPROC,
    // constantes de fenêtre/affichage
    CW_USEDEFAULT, EN_CHANGE, ES_AUTOHSCROLL, GWLP_USERDATA, GWLP_WNDPROC, GWL_EXSTYLE, GW_OWNER, HWND_MESSAGE,
    HWND_TOPMOST, IDC_ARROW, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, MSGFLT_ALLOW,
    PM_REMOVE, SWP_NOZORDER, SW_HIDE, SW_RESTORE, SW_SHOW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_COMPOSITED,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP,
    WS_VISIBLE,
    // messages
    WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_APPCOMMAND, WM_CLIPBOARDUPDATE, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_NCCREATE, WM_NCLBUTTONDOWN, WM_PAINT, WM_QUIT, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_SETCURSOR, WM_SETFONT, WM_TIMER,
};
// WM_MOUSELEAVE est générée par TrackMouseEvent (voir KeyboardAndMouse
// ci-dessous) mais windows-sys la range sous son module "Controls" plutôt
// que "WindowsAndMessaging".
pub use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
// SendInput et les structures INPUT/KEYBDINPUT/MOUSEINPUT ne sont
// délibérément pas ré-exportés : les touches média passent par
// WM_APPCOMMAND vers Shell_TrayWnd (voir core::media), une entrée simulée
// n'atteignant de toute façon que la fenêtre au premier plan et non la
// session média active. Les lier laisserait dans le binaire une capacité
// d'injection clavier inutilisée, que les heuristiques antivirus associent
// aux keyloggers.
pub use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyState, RegisterHotKey, SetActiveWindow, SetFocus, TrackMouseEvent, UnregisterHotKey,
    TME_LEAVE, TRACKMOUSEEVENT, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_0, VK_1, VK_2, VK_3,
    VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_BACK, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_F1,
    VK_LEFT, VK_OEM_MINUS, VK_OEM_PLUS, VK_RETURN, VK_RIGHT, VK_S, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP, VK_W,
};
pub use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE,
};
pub use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
    OpenClipboard, RegisterClipboardFormatW, RemoveClipboardFormatListener, SetClipboardData,
};
// Compte d'objets GDI/USER du process, utilisé par le seul stress test de
// détection de fuites de handles : gated cfg(test) pour ne pas apparaître
// dans le binaire livré.
#[cfg(test)]
pub use windows_sys::Win32::System::Threading::{GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS};

/// CF_UNICODETEXT -- vit dans le module `System::Ole` de windows-sys (donc
/// derrière une feature COM qu'on ne veut pas activer juste pour cette
/// seule constante stable et documentée depuis toujours).
pub const CF_UNICODETEXT: u32 = 13;
pub use windows_sys::Win32::Graphics::Gdi::HBRUSH;

pub const MONITOR_DEFAULTTONEAREST: u32 = 2;
