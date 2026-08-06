//! user32.dll -- fenêtres, messages, entrée, hotkeys, moniteurs, DPI.
//! Ré-exporté depuis windows-sys ; les types de handles (HICON, HCURSOR,
//! HMENU) viennent du même module `WindowsAndMessaging` où windows-sys les
//! définit.

pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
    DispatchMessageW, DrawIconEx, EnumWindows, FindWindowW, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetMessageW, GetParent, GetSystemMetrics,
    GetWindow, GetWindowLongPtrW, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, KillTimer, LoadCursorW, LoadIconW,
    LoadImageW,
    MessageBoxW, MoveWindow, PeekMessageW, PostMessageW, PostQuitMessage, PostThreadMessageW, RegisterClassExW,
    SendMessageW, SetCursor, SetForegroundWindow, SetProcessDPIAware, SetTimer, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, TranslateMessage, UnregisterClassW, CREATESTRUCTW, HCURSOR, HICON, HMENU,
    WNDCLASSEXW, WNDENUMPROC, WNDPROC,
    // constantes de fenêtre/affichage
    CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, EN_CHANGE, ES_AUTOHSCROLL, ES_READONLY, ES_RIGHT, GWLP_USERDATA,
    GWLP_WNDPROC, GWL_EXSTYLE, GWL_STYLE, GW_OWNER, HWND_MESSAGE, HWND_TOPMOST, IDC_ARROW, IDC_HAND,
    IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, MB_ICONERROR, MB_OK, MSGFLT_ALLOW, PM_REMOVE,
    SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE,
    SW_RESTORE, SW_SHOW, SW_SHOWNOACTIVATE, WS_BORDER, WS_CHILD, WS_CLIPCHILDREN, WS_EX_COMPOSITED, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
    CW_USEDEFAULT,
    // messages
    WA_INACTIVE, WM_ACTIVATE, WM_ACTIVATEAPP, WM_APP, WM_APPCOMMAND, WM_CHAR, WM_CLOSE, WM_COMMAND, WM_CTLCOLOREDIT,
    WM_CTLCOLORSTATIC, WM_DESTROY,
    WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
    WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_NCLBUTTONDOWN, WM_PAINT, WM_QUIT,
    WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFOCUS, WM_SETFONT, WM_SHOWWINDOW,
    WM_SYSKEYDOWN, WM_TIMER,
};
// WM_MOUSELEAVE est générée par TrackMouseEvent (voir KeyboardAndMouse
// ci-dessous) mais windows-sys la range sous son module "Controls" plutôt
// que "WindowsAndMessaging".
pub use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
pub use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetFocus, GetKeyState, RegisterHotKey, ReleaseCapture, SendInput, SetActiveWindow, SetCapture,
    SetFocus, TrackMouseEvent, UnregisterHotKey, HARDWAREINPUT, INPUT, INPUT_0, KEYBDINPUT, MOUSEINPUT,
    TRACKMOUSEEVENT, INPUT_HARDWARE, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MOD_ALT,
    MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, TME_LEAVE, VK_A, VK_BACK, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN,
    VK_ESCAPE, VK_F1, VK_LEFT, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU,
    VK_RETURN, VK_RIGHT, VK_S, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
    VK_W,
};
pub use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwareness, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, PROCESS_PER_MONITOR_DPI_AWARE,
};
pub use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
// Utilisé uniquement par le stress test (compte d'objets GDI/USER du
// process, détecte les fuites de handles) -- jamais appelé par l'appli
// elle-même.
pub use windows_sys::Win32::System::Threading::{GetGuiResources, GR_GDIOBJECTS, GR_USEROBJECTS};

/// CF_UNICODETEXT -- vit dans le module `System::Ole` de windows-sys (donc
/// derrière une feature COM qu'on ne veut pas activer juste pour cette
/// seule constante stable et documentée depuis toujours).
pub const CF_UNICODETEXT: u32 = 13;
pub use windows_sys::Win32::Graphics::Gdi::{HBRUSH, HMONITOR};

pub const MONITOR_DEFAULTTONEAREST: u32 = 2;
