//! La fenêtre popup principale : géométrie, contrôle EDIT natif pour la
//! recherche, rendu GDI de la liste de résultats, entrée clavier, tous les
//! modes (Window Switcher, Timer+rebond, Sticky Notes, Auto-restart,
//! sélecteur de thème).
//!
//! Géométrie : largeur = fraction de l'écran (themes.json), hauteur =
//! largeur * 9/16 strictement, contenu divisé en tranches égales (2 pour
//! la barre de recherche + 10 pour les résultats) -- entièrement dérivé de
//! la taille de fenêtre, jamais de dimensions fixes.
//!
//! Tous les modes partagent la même primitive de liste : `mode_items`
//! (les libellés à filtrer/afficher) + `filtered` (les indices retenus,
//! triés) + `selected`/`first_visible`. Seul ce qui remplit `mode_items`
//! et ce que fait Entrée/Suppr changent d'un mode à l'autre -- un seul
//! chemin de rendu/filtrage plutôt que cinq quasi-identiques.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::core::calculator;
use crate::core::emoji::EmojiData;
use crate::core::models::App;
use crate::core::recycle_bin::RecycleBinItem;
use crate::core::search::{match_rank_multi, normalize};
use crate::core::supervisor::RestartSupervisor;
use crate::core::windows::WindowEntry;
use crate::win32::gdi32::{
    BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetMonitorInfoW, InvalidateRect,
    MonitorFromPoint, ReleaseDC, SelectObject, SetBkColor, SetBkMode, SetTextColor, UpdateWindow, HDC, HFONT,
    MONITORINFO, OPAQUE, PAINTSTRUCT, TRANSPARENT, DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
};
use crate::win32::user32::{
    CallWindowProcW, CreateWindowExW, DefWindowProcW, GetClientRect, GetCursorPos, GetKeyState, GetParent,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible, KillTimer, LoadCursorW,
    RegisterClassExW, SendMessageW, SetActiveWindow, SetCursor, SetFocus, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, HBRUSH, WNDPROC,
    EN_CHANGE, ES_AUTOHSCROLL, ES_READONLY, ES_RIGHT, GWLP_USERDATA, GWLP_WNDPROC, HWND_TOPMOST, IDC_ARROW,
    MONITOR_DEFAULTTONEAREST, SWP_NOZORDER, SW_HIDE, SW_SHOW, VK_A, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN, VK_ESCAPE,
    VK_LEFT, VK_RETURN, VK_RIGHT, VK_S, VK_SHIFT, VK_TAB, VK_UP, VK_W, WA_INACTIVE, WM_ACTIVATE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN, WM_PAINT,
    WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFONT, WM_TIMER, WS_CHILD, WS_CLIPCHILDREN,
    WS_EX_COMPOSITED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use crate::win32::{from_wstring, set_clipboard_text, to_wstring, HWND, LPARAM, LRESULT, POINT, RECT, UINT, WPARAM};
use super::gdi::{make_font, rect, rect_h, rect_w, simple_wndclass};

use super::theme::{self, ThemeConfig};

pub const VISIBLE_ROWS: usize = 10;
/// Nombre de "tranches" égales du contenu : 2 pour la barre de recherche
/// (volontairement deux fois plus grande qu'une ligne de résultat) +
/// VISIBLE_ROWS pour les résultats -- toute la géométrie en découle.
const CONTENT_UNITS: i32 = VISIBLE_ROWS as i32 + 2;
/// Fraction de la largeur de la barre de recherche réservée à l'horloge
/// quand `show_clock` est actif.
const CLOCK_WIDTH_FRACTION: f64 = 0.22;
/// Ratio de la hauteur de fenêtre vs largeur -- 16:9 strict, imposé par
/// l'utilisateur plutôt que dérivé d'une mesure de contenu.
const HEIGHT_RATIO: f64 = 9.0 / 16.0;
/// Position verticale de la fenêtre : fraction de la hauteur utile de
/// l'écran depuis le haut -- choix de présentation, pas une contrainte.
const VERTICAL_POSITION_RATIO: f64 = 0.28;

const WINDOW_CLASS_NAME: &str = "MAGILauncherPopupClass";

const CLOCK_TIMER_ID: usize = 1;
const COUNTDOWN_TIMER_ID: usize = 2;
const FIRE_TIMER_ID: usize = 3;
const BOUNCE_TIMER_ID: usize = 4;
const BOUNCE_INTERVAL_MS: u32 = 16;
/// px/tick à ~60fps (16ms) -- vitesse calée à l'œil sur le vrai
/// "Bouncing DVD Logo" (écran de veille Windows historique), qui n'a pas
/// de vitesse officielle documentée en px/tick. Toujours une valeur fixe
/// en pixels, pas une fraction de la largeur d'écran -- le rythme visé est
/// une vitesse constante à l'œil, pas relative à la taille du moniteur.
const BOUNCE_SPEED_PX: f64 = 26.0;

// Entrées spéciales de apps.json (voir README "Special entries").
const SENTINEL_RELOAD: &str = "magi:reload";
const SENTINEL_THEME_PICKER: &str = "magi:theme-picker";
const SENTINEL_TIMER: &str = "magi:timer";
const SENTINEL_NOTES: &str = "magi:notes";
const SENTINEL_RESTART: &str = "magi:auto-restart";
const SENTINEL_OPEN_FOLDER: &str = "magi:open-folder";
const SENTINEL_EMPTY_RECYCLE_BIN: &str = "magi:empty-recycle-bin";
const SENTINEL_MEDIA_PLAY_PAUSE: &str = "magi:media-play-pause";
const SENTINEL_MEDIA_NEXT: &str = "magi:media-next";
const SENTINEL_MEDIA_PREVIOUS: &str = "magi:media-previous";
const SENTINEL_MEDIA_STOP: &str = "magi:media-stop";
const SENTINEL_MEDIA_VOLUME_MUTE: &str = "magi:media-volume-mute";
const SENTINEL_MEDIA_VOLUME_DOWN: &str = "magi:media-volume-down";
const SENTINEL_MEDIA_VOLUME_UP: &str = "magi:media-volume-up";
const SENTINEL_EMOJI: &str = "magi:emoji";

/// Remplace `theme.placeholder_text` tant qu'on est dans la saisie de durée
/// du Timer -- le placeholder générique ("Type to search...") n'a aucun
/// sens ici, contrairement aux autres modes (Window Switcher, Notes...) qui
/// restent bien des recherches et gardent donc le placeholder du thème.
const TIMER_PLACEHOLDER: &str = "Type a duration (5m, 90s, 1h...)";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Normal,
    Window,
    Timer,
    Notes,
    Restart,
    Theme,
    RecycleBin,
    Emoji,
}

#[derive(Clone, Copy, Default)]
struct Geometry {
    window: RECT,
    /// Rects ci-dessous en coordonnées CLIENTES (0,0 = coin haut-gauche de
    /// la fenêtre), un seul jeu de calculs partagé par le repositionnement
    /// des contrôles enfants et par le rendu GDI.
    search: RECT,
    /// Vide (largeur nulle) quand `show_clock` est désactivé.
    clock: RECT,
    separator: RECT,
    rows: [RECT; VISIBLE_ROWS],
}

/// Marge générale unique que TOUT texte doit respecter (lignes de
/// résultat, "Type to search", horloge) -- tout le reste (blocs, rects,
/// contrôles) va bord à bord jusqu'ici ; seule cette marge insère du vide
/// avant le texte lui-même. Un seul point de calcul plutôt qu'un ratio
/// dupliqué à chaque endroit qui dessine du texte -- exactement le genre
/// d'endroit où deux copies avaient fini par diverger (troncature vs
/// arrondi), décalant "Type to search" d'un pixel par rapport aux lignes.
fn text_margin_px(row_h: i32) -> i32 {
    (row_h as f64 * 0.3) as i32
}

/// Rect du CONTRÔLE natif (EDIT recherche ou horloge) à l'intérieur de son
/// bloc visuel -- le bloc lui-même fait deux fois la hauteur d'une ligne
/// (voir compute_geometry), mais le contrôle réel garde une hauteur
/// proche d'une ligne normale (`control_h`) et est centré dedans. Un EDIT
/// single-line étiré à une hauteur très disproportionnée par rapport à sa
/// police ne centre plus fiablement son curseur clignotant par rapport au
/// texte (constaté à l'usage) -- un contrôle à taille normale, positionné
/// à la main au milieu du bloc, centre correctement caret et texte comme
/// n'importe quel champ de recherche standard.
fn centered_control_rect(block: &RECT, control_h: i32) -> RECT {
    let top = block.top + (rect_h(block) - control_h) / 2;
    rect(block.left, top, rect_w(block), control_h)
}

/// Rects (recherche, horloge) des CONTRÔLES réels -- calcul partagé par
/// `create` (position initiale) et `apply_geometry` (repositionnement au
/// Reload/changement de moniteur), pour ne jamais avoir à resynchroniser
/// deux copies de la même formule.
fn search_control_rects(geometry: &Geometry) -> (RECT, RECT) {
    let control_h = rect_h(&geometry.rows[0]);
    (centered_control_rect(&geometry.search, control_h), centered_control_rect(&geometry.clock, control_h))
}

/// Petit générateur pseudo-aléatoire (xorshift64*) -- seulement pour la
/// direction/vitesse initiale du rebond DVD et le choix du prochain thème
/// à chaque collision. Aucun besoin cryptographique, donc pas de raison de
/// tirer une dépendance externe pour ça.
struct SimpleRng(u64);

impl SimpleRng {
    fn new() -> Self {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x853c49e6);
        SimpleRng(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn next_range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Flottant uniforme dans [0, 1) -- utilisé pour tirer l'angle initial
    /// du rebond DVD (voir start_bounce).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Zone de travail (écran moins barre des tâches) du moniteur SOUS LE
/// CURSEUR -- pas forcément le moniteur principal, comportement multi-
/// écran façon Rofi : la popup s'ouvre toujours là où est la souris.
fn work_area_under_cursor() -> RECT {
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

fn compute_geometry(work: RECT, theme: &ThemeConfig) -> Geometry {
    let window_w = (rect_w(&work) as f64 * theme.window_width_fraction).round() as i32;
    let window_h = (window_w as f64 * HEIGHT_RATIO).round() as i32;
    let window_x = work.left + (rect_w(&work) - window_w) / 2;
    let window_y = work.top + (rect_h(&work) as f64 * VERTICAL_POSITION_RATIO).round() as i32;

    let border = theme.border_width.max(1);
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
    // La division entière de unit_h laisse un reste (quelques pixels) --
    // sans l'absorber quelque part, ce reste s'ajoute à la bordure du bas
    // (jamais couverte par aucune ligne), qui paraît alors plus épaisse
    // que les trois autres côtés. La dernière ligne l'absorbe à la place,
    // pour que la bordure du bas fasse exactement la même épaisseur que
    // les autres.
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

enum SearchDisplay {
    List,
    Calc(String),
    Color(u32),
    /// Une seule ligne mise en avant -- aperçu/compte à rebours du Timer.
    SingleLine(String),
}

struct AppState {
    apps: Vec<App>,
    windows: Vec<WindowEntry>,
    /// Reflète le contenu actuel de la Corbeille (voir Mode::RecycleBin) --
    /// `mode_items` n'en dérive que les noms pour l'affichage/filtrage ;
    /// gardé à part comme `windows` pour retrouver le chemin `$I`/`$R` réel
    /// d'un élément sélectionné (voir on_delete/launch_selected).
    recycle_bin_items: Vec<RecycleBinItem>,
    /// `None` si emoji-test.txt est absent/illisible à côté de l'exe (voir
    /// core::emoji::load) -- le mode Emoji reste alors inatteignable
    /// (Entrée sur l'entrée magi:emoji ne fait rien, voir
    /// launch_selected_normal) plutôt que de planter ou d'ouvrir un picker
    /// vide sans explication.
    emoji: Option<EmojiData>,
    notes: Vec<String>,
    notes_path: PathBuf,
    restart_targets: Vec<String>,
    restart_path: PathBuf,
    restart_supervisor: RestartSupervisor,
    /// Reflète si `restart_supervisor` tourne -- exposé au tray (voir
    /// "Disable Auto-restart") pour offrir le même bascule que le hotkey.
    auto_restart_enabled: bool,

    mode: Mode,
    /// Libellés actuellement filtrables/affichables -- reflète `apps` en
    /// mode Normal, `windows`/`notes`/`restart_targets`/les noms de thème
    /// dans les autres modes. Un seul chemin de filtrage/rendu pour tous
    /// les modes (voir le commentaire en tête de fichier).
    mode_items: Vec<String>,
    filtered: Vec<usize>,
    selected: usize,
    first_visible: usize,
    display: SearchDisplay,

    theme: ThemeConfig,
    theme_picker_original: Option<String>,
    base_dir: PathBuf,
    themes_path: PathBuf,

    // Timer + rebond DVD
    timer_deadline: Option<Instant>,
    timer_total_seconds: u64,
    bouncing: bool,
    bounce_pos: (f64, f64),
    bounce_vel: (f64, f64),
    bounce_pre_geometry: Option<Geometry>,
    bounce_pre_theme: Option<String>,
    rng: SimpleRng,

    edit_hwnd: HWND,
    clock_hwnd: HWND,
    geometry: Geometry,
    font_row: HFONT,
    font_search: HFONT,
    search_brush: HBRUSH,
    /// Texte d'invite ("Type to search"), dessiné à la main dans
    /// edit_subclass_proc (voir son commentaire) -- gardé en UTF-16 tout
    /// prêt pour DrawTextW plutôt que reconverti à chaque repaint.
    placeholder_wide: Vec<u16>,
    /// Marge gauche explicitement fixée sur edit_hwnd/clock_hwnd via
    /// EM_SETMARGINS (voir apply_theme_visuals) -- réutilisée telle quelle
    /// par draw_placeholder pour démarrer exactement là où démarre le vrai
    /// texte tapé.
    text_margin: i32,
    /// Cache de `recycle_bin::query()` (compte + taille) -- ce Cell (donc
    /// modifiable même à travers un &AppState partagé, voir row_label)
    /// évite de rappeler cette requête Shell à CHAQUE WM_PAINT tant que la
    /// ligne Corbeille est visible : SHQueryRecycleBinW touche le disque et
    /// peut prendre plusieurs dizaines de ms, largement assez pour geler la
    /// pompe de messages le temps d'une frame à chaque frappe/navigation et
    /// faire apparaître le curseur "chargement" de Windows (ghosting sur
    /// une fenêtre qui met du temps à répondre).
    recycle_bin_cache: std::cell::Cell<Option<(Instant, i64, i64)>>,
}

/// Durée de validité du cache ci-dessus -- assez court pour qu'un vidage de
/// Corbeille (magi:empty-recycle-bin) se reflète vite, assez long pour
/// qu'une rafale de repaints (navigation clavier, rebond DVD) ne déclenche
/// qu'une requête réelle plutôt qu'une par frame.
const RECYCLE_BIN_CACHE_TTL: Duration = Duration::from_secs(2);

fn recycle_bin_cached(state: &AppState) -> (i64, i64) {
    if let Some((at, count, size)) = state.recycle_bin_cache.get() {
        if at.elapsed() < RECYCLE_BIN_CACHE_TTL {
            return (count, size);
        }
    }
    let (count, size) = crate::core::recycle_bin::query();
    state.recycle_bin_cache.set(Some((Instant::now(), count, size)));
    (count, size)
}

/// Vide la Corbeille, invalide le cache count/taille et cache le lanceur --
/// les trois seuls sites qui déclenchent un vidage complet (main list
/// Shift+Entrée/Suppr, et Shift+Suppr depuis la vue de consultation).
/// Invalider le cache ici plutôt que de laisser RECYCLE_BIN_CACHE_TTL
/// expirer de lui-même : le vidage est asynchrone (voir empty_async), mais
/// rien n'oblige le cache à survivre à l'action qui vient de le rendre
/// obsolète -- sans ça, rouvrir le lanceur juste après montrait encore
/// l'ancien compte/poids.
unsafe fn empty_recycle_bin_and_hide(hwnd: HWND, state: &AppState) {
    crate::core::recycle_bin::empty_async();
    state.recycle_bin_cache.set(None);
    hide(hwnd);
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    ptr.as_mut()
}

/// Recalcule `placeholder_wide` selon le mode courant -- TIMER_PLACEHOLDER
/// en mode Timer, sinon le placeholder générique du thème actif. Appelée à
/// la fois par apply_theme_visuals (un changement de thème ne doit pas
/// écraser un placeholder de mode par le générique) et par enter_mode (un
/// changement de mode seul, sans rechargement de thème, doit quand même
/// mettre à jour le texte affiché).
fn refresh_placeholder(state: &mut AppState) {
    let text = if state.mode == Mode::Timer { TIMER_PLACEHOLDER } else { state.theme.placeholder_text.as_str() };
    state.placeholder_wide = to_wstring(text);
}

/// Reconstruit le look de la fenêtre (polices, pinceau de la barre de
/// recherche) à partir de `state.theme.current` -- appelé à la création
/// et à chaque changement de thème. Deux polices distinctes (recherche /
/// lignes de résultat), la police de recherche restant légèrement plus
/// grande (voir son commentaire) que celle des lignes.
unsafe fn apply_theme_visuals(hwnd: HWND, state: &mut AppState) {
    if !state.font_row.is_null() {
        DeleteObject(state.font_row as _);
    }
    if !state.font_search.is_null() {
        DeleteObject(state.font_search as _);
    }
    if !state.search_brush.is_null() {
        DeleteObject(state.search_brush as _);
    }
    let row_h = rect_h(&state.geometry.rows[0]);
    let family = theme::resolve_font_family(&state.theme);
    let row_font_px = (row_h as f64 * 0.6) as i32;
    state.font_row = make_font(&family, row_font_px);
    // 1.2x la police des lignes (pas 2x) -- le bloc de recherche/horloge
    // fait deux fois la hauteur d'une ligne pour la géométrie/le fond,
    // mais le CONTRÔLE EDIT lui-même garde une hauteur proche d'une ligne
    // normale et est centré dedans (voir centered_control_rect) : un
    // ratio de police trop agressif rendait le texte disproportionné par
    // rapport au reste de la liste.
    state.font_search = make_font(&family, (row_font_px as f64 * 1.2) as i32);
    state.search_brush = CreateSolidBrush(state.theme.current.search_background);

    SendMessageW(state.edit_hwnd, WM_SETFONT, state.font_search as usize, 1);
    // Marge gauche/droite fixée explicitement (voir EM_SETMARGINS) plutôt
    // que de dépendre d'une marge implicite non lisible -- MÊME fonction
    // text_margin_px que draw_row_text, pas un ratio recopié à la main :
    // une seule marge générale que tout texte respecte, garantie identique
    // partout par construction plutôt que par deux formules qui doivent
    // rester manuellement synchronisées.
    state.text_margin = text_margin_px(row_h);
    let margins_lparam = state.text_margin as isize | ((state.text_margin as isize) << 16);
    SendMessageW(state.edit_hwnd, EM_SETMARGINS, EC_LEFTMARGIN | EC_RIGHTMARGIN, margins_lparam);
    // Le texte d'invite ("Type to search") n'utilise PAS EM_SETCUEBANNER :
    // ce message dessine son texte avec une couleur interne à comctl32
    // (grisé système), qui ignore totalement SetTextColor -- il ne suivait
    // donc jamais la couleur `search_text` du thème (contrairement à
    // l'horloge, qui affiche du vrai texte via WM_CTLCOLOREDIT). À la
    // place, `placeholder_wide` est dessiné à la main dans
    // edit_subclass_proc (WM_PAINT) quand le champ est vide, avec la
    // police et la couleur du thème comme n'importe quel autre texte.
    refresh_placeholder(state);

    if !state.clock_hwnd.is_null() {
        SendMessageW(state.clock_hwnd, WM_SETFONT, state.font_search as usize, 1);
        SendMessageW(state.clock_hwnd, EM_SETMARGINS, EC_LEFTMARGIN | EC_RIGHTMARGIN, margins_lparam);
    }

    // Les contrôles enfants (EDIT/STATIC) ont leur propre cycle de peinture
    // indépendant de celui de la fenêtre parente -- les invalider ici
    // force leur repaint immédiat avec les NOUVELLES couleurs (via
    // WM_CTLCOLOREDIT/WM_CTLCOLORSTATIC) dès qu'un thème change ; sans ça,
    // ils gardaient l'ancienne couleur de fond jusqu'à leur prochain
    // repaint naturel (perte de focus, frappe...), ce qui se voyait comme
    // un morceau de l'ancien thème resté visible derrière la nouvelle
    // liste pendant une preview.
    InvalidateRect(state.edit_hwnd, std::ptr::null(), 1);
    if !state.clock_hwnd.is_null() {
        InvalidateRect(state.clock_hwnd, std::ptr::null(), 1);
    }
    let _ = hwnd;
}

fn get_edit_text(edit_hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(edit_hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        GetWindowTextW(edit_hwnd, buf.as_mut_ptr(), buf.len() as i32);
        from_wstring(&buf)
    }
}

fn set_edit_text(edit_hwnd: HWND, text: &str) {
    unsafe {
        SetWindowTextW(edit_hwnd, to_wstring(text).as_ptr());
    }
}

// --- Construction des libellés par mode ---------------------------------

fn rebuild_normal_items(state: &mut AppState) {
    state.mode_items = state.apps.iter().map(|a| a.name.clone()).collect();
}

fn rebuild_notes_items(state: &mut AppState) {
    state.mode_items = state.notes.clone();
}

fn rebuild_restart_items(state: &mut AppState) {
    let running: Vec<String> =
        crate::core::supervisor::running_exe_names().iter().map(|s| s.to_lowercase()).collect();
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

/// Contenu de la Corbeille (voir Mode::RecycleBin) -- une seule requête à
/// l'entrée du mode, pas à chaque frappe.
fn rebuild_recyclebin_items(state: &mut AppState) {
    state.recycle_bin_items = crate::core::recycle_bin::list_items();
    state.mode_items = state.recycle_bin_items.iter().map(|item| item.name.clone()).collect();
}

fn rebuild_window_items(state: &mut AppState) {
    state.windows = crate::core::windows::list_windows();
    state.mode_items = state.windows.iter().map(|w| w.title.clone()).collect();
}

fn rebuild_theme_items(state: &mut AppState) {
    state.mode_items = theme::list_theme_names(&state.theme);
}

fn rebuild_emoji_items(state: &mut AppState) {
    state.mode_items =
        state.emoji.as_ref().map(|d| d.entries.iter().map(|e| e.name.clone()).collect()).unwrap_or_default();
}

/// Libellé réellement affiché pour l'entrée `idx` du mode courant -- dérive
/// de `mode_items`, sauf deux cas où l'original affiche une valeur "en
/// direct" recalculée à chaque rendu plutôt qu'un texte figé : le compte
/// de la Corbeille (mode Normal) et le compte à rebours du Timer déjà géré
/// séparément (mode Timer, voir SearchDisplay::SingleLine).
fn row_label(state: &AppState, idx: usize) -> String {
    if state.mode == Mode::Normal {
        if let Some(app) = state.apps.get(idx) {
            if app.path == SENTINEL_EMPTY_RECYCLE_BIN {
                let (count, size) = recycle_bin_cached(state);
                return if count > 0 {
                    format!("{}: {} items, {:.1} MB", app.name, count, size as f64 / 1_048_576.0)
                } else {
                    app.name.clone()
                };
            }
            // Toujours un suffixe "<nom>: ..." même à l'état vide/arrêté
            // (comme le sélecteur de thème juste en dessous) -- une entrée
            // qui affiche parfois juste son nom brut et parfois "nom: état"
            // donnerait l'impression trompeuse qu'il ne se passe jamais
            // rien tant qu'aucun timer/cible/note n'a encore été ajouté.
            if app.path == SENTINEL_TIMER {
                let value = match state.timer_deadline {
                    Some(deadline) => {
                        let remaining = (deadline - Instant::now()).as_secs() as i64;
                        crate::core::timer::format_remaining(remaining)
                    }
                    None => "--:--".to_string(),
                };
                return format!("{}: {}", app.name, value);
            }
            // La plus récente en premier (voir launch_selected : insert(0, ..)),
            // donc notes[0] est bien la dernière ajoutée.
            if app.path == SENTINEL_NOTES {
                return match state.notes.first() {
                    Some(latest) => format!("{}: {}", app.name, latest),
                    None => format!("{}:", app.name),
                };
            }
            if app.path == SENTINEL_RESTART {
                return format!("{}: {}", app.name, state.restart_targets.len());
            }
            if app.path == SENTINEL_THEME_PICKER {
                return format!("{}: {}", app.name, state.theme.active_theme);
            }
            if app.path == SENTINEL_EMOJI {
                let status = match &state.emoji {
                    Some(data) => format!("Version {}", data.version),
                    None => "missing emoji-test.txt".to_string(),
                };
                return format!("{}: {}", app.name, status);
            }
        }
    }
    // "<emoji> <nom>" à l'affichage -- mode_items (utilisé pour le
    // filtrage, voir fuzzy_filter) ne contient volontairement QUE le nom,
    // sans le glyphe en préfixe : sinon "gri" ne matcherait plus "grinning
    // face" en préfixe (tier 0) puisque la chaîne comparée commencerait par
    // l'emoji, pas par le nom.
    if state.mode == Mode::Emoji {
        if let Some(entry) = state.emoji.as_ref().and_then(|d| d.entries.get(idx)) {
            return format!("{} {}", entry.glyph, entry.name);
        }
    }
    state.mode_items.get(idx).cloned().unwrap_or_default()
}

// --- Filtrage -------------------------------------------------------------

fn fuzzy_filter(items: &[String], query_lower: &str) -> Vec<usize> {
    if query_lower.is_empty() {
        return (0..items.len()).collect();
    }
    let mut ranked: Vec<(usize, (u8, usize))> = items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match_rank_multi(&normalize(s), query_lower).map(|r| (i, r)))
        .collect();
    ranked.sort_by(|a, b| a.1.cmp(&b.1));
    ranked.into_iter().map(|(i, _)| i).collect()
}

/// Réévalue le mode d'affichage et le classement à partir du texte actuel
/// de l'EDIT. En mode Normal, même ordre de priorité que le README :
/// couleur hex, puis expression arithmétique, puis recherche floue.
fn refresh_filter(state: &mut AppState) {
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
    state.filtered = fuzzy_filter(&state.mode_items, &normalize(trimmed));
}

fn current_list_len(state: &AppState) -> usize {
    match state.display {
        SearchDisplay::List => state.filtered.len(),
        _ => 0,
    }
}

unsafe fn move_selection(hwnd: HWND, state: &mut AppState, delta: i32) {
    let len = current_list_len(state);
    if len == 0 {
        return;
    }
    let new_selected = (state.selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
    state.selected = new_selected;
    if state.selected < state.first_visible {
        state.first_visible = state.selected;
    } else if state.selected >= state.first_visible + VISIBLE_ROWS {
        state.first_visible = state.selected - VISIBLE_ROWS + 1;
    }
    if state.mode == Mode::Theme {
        if let Some(&idx) = state.filtered.get(state.selected) {
            if let Some(name) = state.mode_items.get(idx).cloned() {
                theme::preview_theme(&mut state.theme, &name);
                // Sans ça, la barre de recherche (police/marges/pinceau/
                // placeholder) ne se mettait à jour qu'à la prochaine
                // frappe de texte ou tick d'horloge -- les contrôles EDIT
                // ont leur propre cycle de peinture indépendant de la
                // fenêtre principale (voir apply_theme_visuals), donc une
                // preview en direct doit explicitement les redessiner à
                // CHAQUE flèche, pas seulement à la validation/sortie.
                apply_theme_visuals(hwnd, state);
            }
        }
    }
}

// --- Changement de mode ---------------------------------------------------

unsafe fn enter_mode(hwnd: HWND, state: &mut AppState, mode: Mode) {
    if mode == Mode::Theme {
        state.theme_picker_original = Some(state.theme.active_theme.clone());
    }
    state.mode = mode;
    match mode {
        Mode::Normal => rebuild_normal_items(state),
        Mode::Window => rebuild_window_items(state),
        Mode::Notes => rebuild_notes_items(state),
        Mode::Restart => rebuild_restart_items(state),
        Mode::Theme => rebuild_theme_items(state),
        Mode::RecycleBin => rebuild_recyclebin_items(state),
        Mode::Emoji => rebuild_emoji_items(state),
        Mode::Timer => {}
    }
    set_edit_text(state.edit_hwnd, "");
    refresh_placeholder(state);
    InvalidateRect(state.edit_hwnd, std::ptr::null(), 1);
    refresh_filter(state);
    if mode == Mode::Theme {
        // refresh_filter remet toujours la sélection à l'index 0 (le
        // premier thème dans l'ordre alphabétique de list_theme_names) --
        // sans ce recalage, ouvrir le picker montrait presque toujours un
        // thème DIFFÉRENT du thème actif comme "sélectionné", plutôt que de
        // partir de là où on est déjà.
        if let Some(idx) = state.mode_items.iter().position(|name| *name == state.theme.active_theme) {
            state.selected = idx;
            state.first_visible = if idx >= VISIBLE_ROWS { idx - VISIBLE_ROWS + 1 } else { 0 };
        }
    }
    let _ = hwnd;
}

/// Revient au mode Normal -- annule une preview de thème en cours si on
/// sort du sélecteur sans avoir validé (Entrée), restaure le thème actif
/// d'origine.
/// Annule une preview de thème non validée si on est en train de quitter
/// le sélecteur -- partagé par `exit_picker` (Échap, retour au menu
/// principal) et Tab (Window Switcher accessible depuis n'importe quel
/// mode, voir handle_edit_keydown) : quitter Thème sans valider ne doit
/// jamais laisser un thème juste prévisualisé collé, peu importe vers où
/// on va ensuite.
unsafe fn cancel_uncommitted_theme_preview(hwnd: HWND, state: &mut AppState) {
    if state.mode == Mode::Theme {
        if let Some(orig) = state.theme_picker_original.take() {
            theme::preview_theme(&mut state.theme, &orig);
            apply_theme_visuals(hwnd, state);
        }
    }
}

unsafe fn exit_picker(hwnd: HWND, state: &mut AppState) {
    cancel_uncommitted_theme_preview(hwnd, state);
    enter_mode(hwnd, state, Mode::Normal);
}

// --- Actions ---------------------------------------------------------------

unsafe fn reload_config(hwnd: HWND, state: &mut AppState) {
    if let Ok((_, apps)) = crate::core::config::load_config(&state.base_dir.join("apps.json")) {
        state.apps = apps;
    }
    theme::load(&state.themes_path, &mut state.theme);
    // notes.json/restart.json sont normalement toujours à jour en mémoire
    // (le lanceur est seul à les écrire, voir leur commentaire dans
    // core::json_list) -- mais les relire ici aussi coûte peu et couvre le
    // cas d'une modification manuelle de restart.json pendant que le
    // lanceur tourne, qui ne se reflétait sinon jamais sans quitter/
    // relancer l'appli entière.
    state.notes = crate::core::json_list::load_notes(&state.notes_path);
    state.restart_targets = crate::core::json_list::load_restart_list(&state.restart_path);
    state.restart_supervisor.set_targets(state.restart_targets.clone());
    // emoji-test.txt suit la même logique -- reload permet de déposer une
    // version plus récente d'Unicode sans redémarrer le lanceur.
    state.emoji = crate::core::emoji::load(&state.base_dir.join("emoji-test.txt"));
    apply_geometry(hwnd, state);
    enter_mode(hwnd, state, Mode::Normal);
}

unsafe fn launch_selected(hwnd: HWND, state: &mut AppState) {
    match state.mode {
        Mode::Normal => launch_selected_normal(hwnd, state),
        Mode::Window => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(w) = state.windows.get(idx) {
                    crate::core::windows::activate_window(w.hwnd);
                }
            }
            hide(hwnd);
        }
        Mode::Notes => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                // Correspondance existante -> copie et ferme.
                if let Some(note) = state.notes.get(idx).cloned() {
                    set_clipboard_text(hwnd, &note);
                    hide(hwnd);
                    return;
                }
            }
            let query = get_edit_text(state.edit_hwnd);
            if !query.trim().is_empty() {
                state.notes.insert(0, query);
                let _ = crate::core::json_list::save_notes(&state.notes_path, &state.notes);
                rebuild_notes_items(state);
                set_edit_text(state.edit_hwnd, "");
                refresh_filter(state);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Mode::Restart => {
            if !state.filtered.is_empty() {
                return; // cible déjà surveillée -- rien à faire ici (voir Suppr)
            }
            let query = get_edit_text(state.edit_hwnd);
            let target = query.trim().to_string();
            // Pas de validation de format ici (ancienne exigence ".exe"
            // supprimée à la demande de l'utilisateur) -- n'importe quelle
            // cible non vide est acceptée telle quelle, même avec des
            // arguments en plus du chemin ; si elle ne se lance pas, ce
            // sera visible à l'usage plutôt que bloqué a priori.
            if !target.is_empty() {
                state.restart_targets.push(target);
                let _ = crate::core::json_list::save_restart_list(&state.restart_path, &state.restart_targets);
                state.restart_supervisor.set_targets(state.restart_targets.clone());
                rebuild_restart_items(state);
                set_edit_text(state.edit_hwnd, "");
                refresh_filter(state);
            }
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        Mode::Theme => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(name) = state.mode_items.get(idx).cloned() {
                    theme::preview_theme(&mut state.theme, &name);
                    let _ = theme::commit_theme(&state.themes_path, &name);
                    state.theme.active_theme = name;
                    state.theme_picker_original = None;
                    apply_theme_visuals(hwnd, state);
                }
            }
            enter_mode(hwnd, state, Mode::Normal);
        }
        Mode::Timer => {
            if let Some(secs) = crate::core::timer::parse_duration(get_edit_text(state.edit_hwnd).trim()) {
                arm_timer(hwnd, state, secs);
                exit_picker(hwnd, state);
            }
        }
        // Copie le nom complet (avec extension) de l'élément en
        // surbrillance -- vider la Corbeille elle-même reste réservé à
        // Maj+Suppr ici, ou à Maj+Entrée/Suppr sur "Empty Recycle Bin"
        // depuis le menu principal ; jamais cette simple Entrée.
        Mode::RecycleBin => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(item) = state.recycle_bin_items.get(idx) {
                    set_clipboard_text(hwnd, &item.name);
                    hide(hwnd);
                }
            }
        }
        Mode::Emoji => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(entry) = state.emoji.as_ref().and_then(|d| d.entries.get(idx)) {
                    set_clipboard_text(hwnd, &entry.glyph);
                    hide(hwnd);
                }
            }
        }
    }
}

unsafe fn launch_selected_normal(hwnd: HWND, state: &mut AppState) {
    match &state.display {
        SearchDisplay::Calc(text) => {
            let value = text.trim_start_matches("= ").to_string();
            set_clipboard_text(hwnd, &value);
            hide(hwnd);
        }
        SearchDisplay::Color(_) => {
            let query = get_edit_text(state.edit_hwnd);
            set_clipboard_text(hwnd, query.trim());
            hide(hwnd);
        }
        SearchDisplay::SingleLine(_) => {}
        SearchDisplay::List => {
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(app) = state.apps.get(idx) else { return };
            let path = app.path.as_str();
            match path {
                SENTINEL_RELOAD => {
                    reload_config(hwnd, state);
                    hide(hwnd);
                }
                SENTINEL_THEME_PICKER => enter_mode(hwnd, state, Mode::Theme),
                SENTINEL_TIMER => enter_mode(hwnd, state, Mode::Timer),
                SENTINEL_NOTES => enter_mode(hwnd, state, Mode::Notes),
                SENTINEL_RESTART => enter_mode(hwnd, state, Mode::Restart),
                // Rien si emoji-test.txt est absent (state.emoji == None) --
                // pas d'action destructrice ni de picker vide à ouvrir sans
                // explication, l'entrée elle-même l'annonce déjà (voir
                // row_label : "Emoji: missing emoji-test.txt").
                SENTINEL_EMOJI => {
                    if state.emoji.is_some() {
                        enter_mode(hwnd, state, Mode::Emoji);
                    }
                }
                SENTINEL_OPEN_FOLDER => {
                    let _ = crate::core::launch::launch(&state.base_dir.to_string_lossy(), None, false);
                    hide(hwnd);
                }
                // Entrée affiche le contenu de la Corbeille dans le
                // lanceur lui-même (nouveau mode, lecture seule) --
                // Maj+Entrée (reveal_or_edit) la vide directement, l'action
                // destructive réservée au modificateur.
                SENTINEL_EMPTY_RECYCLE_BIN => enter_mode(hwnd, state, Mode::RecycleBin),
                SENTINEL_MEDIA_PLAY_PAUSE => send_media(hwnd, crate::core::media::MediaKey::PlayPause),
                SENTINEL_MEDIA_NEXT => send_media(hwnd, crate::core::media::MediaKey::Next),
                SENTINEL_MEDIA_PREVIOUS => send_media(hwnd, crate::core::media::MediaKey::Previous),
                SENTINEL_MEDIA_STOP => send_media(hwnd, crate::core::media::MediaKey::Stop),
                SENTINEL_MEDIA_VOLUME_MUTE => send_media(hwnd, crate::core::media::MediaKey::VolumeMute),
                SENTINEL_MEDIA_VOLUME_DOWN => send_media(hwnd, crate::core::media::MediaKey::VolumeDown),
                SENTINEL_MEDIA_VOLUME_UP => send_media(hwnd, crate::core::media::MediaKey::VolumeUp),
                _ => {
                    let path = app.path.clone();
                    let cwd = app.cwd.clone();
                    let hidden = app.hidden;
                    hide(hwnd);
                    let _ = crate::core::launch::launch(&path, cwd.as_deref(), hidden);
                }
            }
        }
    }
}

unsafe fn send_media(hwnd: HWND, key: crate::core::media::MediaKey) {
    crate::core::media::send_media_key(key);
    hide(hwnd);
}

/// Maj+Entrée : révèle la cible dans l'Explorateur au lieu de la lancer
/// (mode Normal), ou ouvre notes.json dans son éditeur associé (mode
/// Notes) -- même distinction que l'original.
unsafe fn reveal_or_edit(hwnd: HWND, state: &mut AppState) {
    match state.mode {
        Mode::Notes => {
            let _ = crate::core::launch::launch(&state.notes_path.to_string_lossy(), None, false);
            hide(hwnd);
        }
        Mode::Normal => {
            let SearchDisplay::List = state.display else { return };
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(app) = state.apps.get(idx) else { return };
            if app.path == SENTINEL_EMPTY_RECYCLE_BIN {
                empty_recycle_bin_and_hide(hwnd, state);
                return;
            }
            if !app.path.starts_with("magi:") {
                let path = app.path.clone();
                hide(hwnd);
                let _ = crate::core::launch::reveal_in_explorer(&path);
            }
        }
        _ => {}
    }
}

unsafe fn on_delete(hwnd: HWND, state: &mut AppState, shift: bool) {
    match state.mode {
        Mode::Window => {
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(w) = state.windows.get(idx) else { return };
            if shift {
                crate::core::windows::kill_window(w.hwnd);
            } else {
                crate::core::windows::close_window(w.hwnd);
            }
            // Suppression optimiste de la liste LOCALE plutôt qu'une
            // ré-énumération immédiate (EnumWindows) : close_window
            // s'appuie sur PostMessageW(WM_CLOSE), asynchrone -- une
            // ré-énumération synchrone juste après retrouve quasiment
            // toujours la fenêtre encore vivante (l'appli n'a pas encore
            // eu le temps de traiter WM_CLOSE), donc Suppr semblait n'avoir
            // aucun effet visible dans le picker.
            state.windows.remove(idx);
            state.mode_items = state.windows.iter().map(|w| w.title.clone()).collect();
            refresh_filter(state);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        Mode::Notes => {
            if shift {
                state.notes.clear();
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.notes.len() {
                    state.notes.remove(idx);
                }
            }
            let _ = crate::core::json_list::save_notes(&state.notes_path, &state.notes);
            rebuild_notes_items(state);
            refresh_filter(state);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        Mode::Restart => {
            if shift {
                state.restart_targets.clear();
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.restart_targets.len() {
                    state.restart_targets.remove(idx);
                }
            }
            let _ = crate::core::json_list::save_restart_list(&state.restart_path, &state.restart_targets);
            state.restart_supervisor.set_targets(state.restart_targets.clone());
            rebuild_restart_items(state);
            refresh_filter(state);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        Mode::RecycleBin => {
            if shift {
                empty_recycle_bin_and_hide(hwnd, state);
                return;
            }
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(item) = state.recycle_bin_items.get(idx).cloned() else { return };
            crate::core::recycle_bin::delete_item(&item);
            state.recycle_bin_cache.set(None);
            rebuild_recyclebin_items(state);
            refresh_filter(state);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        Mode::Timer => {
            if state.timer_deadline.is_some() {
                cancel_timer(hwnd, state);
            }
        }
        Mode::Normal => {
            let SearchDisplay::List = state.display else { return };
            let Some(&idx) = state.filtered.get(state.selected) else { return };
            let Some(app) = state.apps.get(idx) else { return };
            if app.path == SENTINEL_EMPTY_RECYCLE_BIN {
                empty_recycle_bin_and_hide(hwnd, state);
            } else if app.path == SENTINEL_TIMER && state.timer_deadline.is_some() {
                cancel_timer(hwnd, state);
            }
        }
        _ => {}
    }
}

// --- Timer / rebond DVD -----------------------------------------------

unsafe fn arm_timer(hwnd: HWND, state: &mut AppState, seconds: u64) {
    state.timer_total_seconds = seconds;
    state.timer_deadline = Some(Instant::now() + Duration::from_secs(seconds));
    KillTimer(hwnd, FIRE_TIMER_ID);
    KillTimer(hwnd, COUNTDOWN_TIMER_ID);
    SetTimer(hwnd, FIRE_TIMER_ID, (seconds.min(u32::MAX as u64 / 1000) as u32).saturating_mul(1000).max(1), None);
    SetTimer(hwnd, COUNTDOWN_TIMER_ID, 1000, None);
}

unsafe fn cancel_timer(hwnd: HWND, state: &mut AppState) {
    state.timer_deadline = None;
    KillTimer(hwnd, FIRE_TIMER_ID);
    KillTimer(hwnd, COUNTDOWN_TIMER_ID);
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

unsafe fn on_timer_fire(hwnd: HWND, state: &mut AppState) {
    KillTimer(hwnd, FIRE_TIMER_ID);
    KillTimer(hwnd, COUNTDOWN_TIMER_ID);
    state.timer_deadline = None;
    enter_mode(hwnd, state, Mode::Normal);
    start_bounce(hwnd, state);
}

unsafe fn start_bounce(hwnd: HWND, state: &mut AppState) {
    state.bouncing = true;
    state.bounce_pre_geometry = Some(state.geometry);
    state.bounce_pre_theme = Some(state.theme.active_theme.clone());
    state.bounce_pos = (state.geometry.window.left as f64, state.geometry.window.top as f64);
    let angle = state.rng.next_f64() * std::f64::consts::TAU;
    state.bounce_vel = (BOUNCE_SPEED_PX * angle.cos(), BOUNCE_SPEED_PX * angle.sin());
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    SetActiveWindow(hwnd);
    SetTimer(hwnd, BOUNCE_TIMER_ID, BOUNCE_INTERVAL_MS, None);
}

unsafe fn stop_bounce(hwnd: HWND, state: &mut AppState) {
    state.bouncing = false;
    KillTimer(hwnd, BOUNCE_TIMER_ID);
    if let Some(name) = state.bounce_pre_theme.take() {
        theme::preview_theme(&mut state.theme, &name);
        apply_theme_visuals(hwnd, state);
    }
    if let Some(geometry) = state.bounce_pre_geometry.take() {
        state.geometry = geometry;
        let g = geometry.window;
        SetWindowPos(hwnd, HWND_TOPMOST, g.left, g.top, rect_w(&g), rect_h(&g), SWP_NOZORDER);
        // Sans ce repaint synchrone immédiat, WS_EX_COMPOSITED (voir
        // create()) laisse la DWM recomposer la fenêtre à sa taille/position
        // d'origine à partir de son ancien tampon hors écran (celui du
        // dernier tick de rebond) le temps qu'un WM_PAINT naturel arrive --
        // visible comme un bref clignotement du bas de la fenêtre avec la
        // couleur selected_background du thème prévisualisé pendant le
        // rebond. Invalider PUIS forcer le traitement immédiat du WM_PAINT
        // (UpdateWindow) garantit que le contenu affiché correspond déjà à
        // la géométrie/au thème restaurés avant que quoi que ce soit
        // d'autre (dont un hide() côté appelant) ne s'exécute.
        InvalidateRect(hwnd, std::ptr::null(), 1);
        UpdateWindow(hwnd);
    }
}

unsafe fn bounce_tick(hwnd: HWND, state: &mut AppState) {
    let work = work_area_under_cursor();
    let (w, h) = (rect_w(&state.geometry.window), rect_h(&state.geometry.window));
    let (mut x, mut y) = state.bounce_pos;
    let (mut vx, mut vy) = state.bounce_vel;
    x += vx;
    y += vy;
    let mut bounced = false;
    if x < work.left as f64 {
        x = work.left as f64;
        vx = -vx;
        bounced = true;
    } else if x + w as f64 > work.right as f64 {
        x = (work.right - w) as f64;
        vx = -vx;
        bounced = true;
    }
    if y < work.top as f64 {
        y = work.top as f64;
        vy = -vy;
        bounced = true;
    } else if y + h as f64 > work.bottom as f64 {
        y = (work.bottom - h) as f64;
        vy = -vy;
        bounced = true;
    }
    state.bounce_pos = (x, y);
    state.bounce_vel = (vx, vy);
    SetWindowPos(hwnd, HWND_TOPMOST, x.round() as i32, y.round() as i32, w, h, SWP_NOZORDER);
    if bounced {
        let names = theme::list_theme_names(&state.theme);
        if !names.is_empty() {
            let pick = state.rng.next_range(names.len());
            theme::preview_theme(&mut state.theme, &names[pick]);
            apply_theme_visuals(hwnd, state);
        }
    }
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

// --- Rendu -------------------------------------------------------------

unsafe fn draw_scene(hdc: HDC, state: &AppState) {
    let g = &state.geometry;
    let t = &state.theme.current;

    // Fond de bordure sur toute la fenêtre, puis la barre de recherche et
    // le séparateur par-dessus (rectangles imbriqués) -- la bordure n'est
    // jamais qu'une couleur de fond qui dépasse.
    fill(hdc, &full_window_rect(g), t.border);
    // Recherche ET horloge forment UN SEUL bloc visuel search_background
    // (voir compute_geometry) -- les deux rects doivent être remplis, pas
    // seulement g.search : sans le fill de g.clock, les franges du bloc
    // horloge non couvertes par le contrôle EDIT réel (centré dedans,
    // voir centered_control_rect) montraient la couleur de bordure du
    // fill plein-fenêtre juste au-dessus, comme des bandes parasites.
    fill(hdc, &g.search, t.search_background);
    fill(hdc, &g.clock, t.search_background);
    fill(hdc, &g.separator, t.border);

    SetBkMode(hdc, TRANSPARENT as i32);
    let old_font = SelectObject(hdc, state.font_row as _);

    match &state.display {
        SearchDisplay::Color(color) => {
            for row in g.rows.iter() {
                fill(hdc, row, *color);
            }
        }
        SearchDisplay::Calc(text) | SearchDisplay::SingleLine(text) => {
            fill(hdc, &g.rows[0], t.selected_background);
            draw_row_text(hdc, &g.rows[0], text, t.selected_text);
            for row in g.rows[1..].iter() {
                fill(hdc, row, t.list_background);
            }
        }
        SearchDisplay::List => {
            for (slot, row) in g.rows.iter().enumerate() {
                let list_index = state.first_visible + slot;
                match state.filtered.get(list_index) {
                    Some(&item_index) => {
                        let selected = list_index == state.selected;
                        let (bg, fg) = if selected {
                            (t.selected_background, t.selected_text)
                        } else {
                            (t.list_background, t.list_text)
                        };
                        fill(hdc, row, bg);
                        draw_row_text(hdc, row, &row_label(state, item_index), fg);
                    }
                    None => fill(hdc, row, t.list_background),
                }
            }
        }
    }

    SelectObject(hdc, old_font);
}

/// Le rectangle de la fenêtre entière, en coordonnées clientes -- basé sur
/// les dimensions RÉELLES de la fenêtre (`g.window`), pas recalculé à
/// partir des lignes : une somme reconstruite à partir des rects de lignes
/// peut sous-estimer de quelques pixels à cause des arrondis de la
/// division entière dans compute_geometry, laissant sinon un liseré tout
/// en bas jamais repeint par aucun fill() -- une bordure "manquante" en
/// apparence alors que c'est juste un pixel non couvert.
fn full_window_rect(g: &Geometry) -> RECT {
    rect(0, 0, rect_w(&g.window), rect_h(&g.window))
}

unsafe fn fill(hdc: HDC, r: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, r, brush);
    DeleteObject(brush as _);
}

unsafe fn draw_row_text(hdc: HDC, row: &RECT, text: &str, color: u32) {
    let pad = text_margin_px(rect_h(row));
    let mut text_rect = rect(row.left + pad, row.top, rect_w(row) - 2 * pad, rect_h(row));
    // Les retours à la ligne (notes collées, cibles...) sont aplatis en
    // espaces avant l'affichage -- une ligne de la liste a une hauteur
    // fixe, jamais prévue pour du texte multi-ligne.
    let flattened = text.replace(['\n', '\r'], " ");
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

// --- Fenêtre / message ---------------------------------------------------

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            // Le fond entier est déjà repeint dans draw_scene à chaque
            // WM_PAINT -- laisser l'effacement par défaut ferait clignoter
            // la fenêtre pour rien.
            1
        }
        WM_PAINT => {
            // Dessiné directement sur le vrai DC -- WS_EX_COMPOSITED (voir
            // create()) délègue le double buffering à la DWM pour toute la
            // fenêtre ET ses enfants (EDIT de recherche, horloge) en une
            // seule fois, plutôt qu'un tampon mémoire manuel ici qui ne
            // couvrirait de toute façon que le contenu propre à cette
            // fenêtre, pas ses contrôles enfants.
            if let Some(state) = get_state(hwnd) {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                draw_scene(hdc, state);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            // Même traitement pour l'EDIT de recherche et le STATIC de
            // l'horloge -- les deux doivent se fondre dans la barre de
            // recherche (même couleur de fond que search_background).
            if let Some(state) = get_state(hwnd) {
                let hdc = wparam as HDC;
                SetTextColor(hdc, state.theme.current.search_text);
                SetBkColor(hdc, state.theme.current.search_background);
                SetBkMode(hdc, OPAQUE as i32);
                return state.search_brush as isize;
            }
            0
        }
        WM_COMMAND => {
            if let Some(state) = get_state(hwnd) {
                let notify_code = ((wparam >> 16) & 0xFFFF) as u32;
                let ctrl_hwnd = lparam as HWND;
                if ctrl_hwnd == state.edit_hwnd && notify_code == EN_CHANGE {
                    refresh_filter(state);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_TIMER => {
            if let Some(state) = get_state(hwnd) {
                match wparam {
                    CLOCK_TIMER_ID => {
                        if !state.clock_hwnd.is_null() {
                            SetWindowTextW(state.clock_hwnd, to_wstring(&crate::core::clock::format_now()).as_ptr());
                        }
                        if state.timer_deadline.is_some() && state.mode == Mode::Normal {
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    COUNTDOWN_TIMER_ID => {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    FIRE_TIMER_ID => on_timer_fire(hwnd, state),
                    BOUNCE_TIMER_ID => bounce_tick(hwnd, state),
                    _ => {}
                }
            }
            0
        }
        // Un clic (n'importe quel bouton) est une des trois sorties du
        // rebond DVD, avec Échap et le raccourci global -- n'a AUCUN effet
        // en dehors du rebond (voir le README : la souris n'agit nulle
        // part ailleurs dans la fenêtre du lanceur), ce bloc ne fait donc
        // rien tant que `bouncing` est faux.
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            if let Some(state) = get_state(hwnd) {
                if state.bouncing {
                    stop_bounce(hwnd, state);
                    hide(hwnd);
                }
            }
            0
        }
        WM_ACTIVATE => {
            // Perte de focus (basculement vers une autre appli) -> ferme
            // la popup, même logique que le FocusOut de l'original.
            if (wparam & 0xFFFF) as u32 == WA_INACTIVE {
                if let Some(state) = get_state(hwnd) {
                    if state.bouncing {
                        stop_bounce(hwnd, state);
                    } else {
                        // Sans ça, une preview de thème en cours (flèches
                        // dans le sélecteur) survivait à une perte de focus
                        // (alt-tab, notification...) -- au prochain
                        // affichage, l'UI restait dans les couleurs d'un
                        // thème jamais validé, comme si Échap n'avait
                        // jamais été pressé.
                        cancel_uncommitted_theme_preview(hwnd, state);
                    }
                }
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_DESTROY => {
            if let Some(state) = get_state(hwnd) {
                KillTimer(hwnd, CLOCK_TIMER_ID);
                KillTimer(hwnd, COUNTDOWN_TIMER_ID);
                KillTimer(hwnd, FIRE_TIMER_ID);
                KillTimer(hwnd, BOUNCE_TIMER_ID);
                state.restart_supervisor.stop();
                if !state.font_row.is_null() {
                    DeleteObject(state.font_row as _);
                }
                if !state.font_search.is_null() {
                    DeleteObject(state.font_search as _);
                }
                if !state.search_brush.is_null() {
                    DeleteObject(state.search_brush as _);
                }
            }
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Procédure d'origine de la classe stock "EDIT", capturée avant de la
/// sous-classer (voir `create`) -- les deux contrôles EDIT de cette
/// fenêtre (recherche + horloge) partagent la même classe système, donc la
/// même adresse de procédure, une seule capture suffit pour les deux.
static ORIGINAL_EDIT_WNDPROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

unsafe fn call_original_edit_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let orig = ORIGINAL_EDIT_WNDPROC.load(std::sync::atomic::Ordering::Relaxed);
    let proc: WNDPROC = std::mem::transmute(orig);
    CallWindowProcW(proc, hwnd, msg, wparam, lparam)
}

/// EM_SETMARGINS -- fixe la marge interne GAUCHE/DROITE d'un EDIT à une
/// valeur EXPLICITE et connue, plutôt que de dépendre de la marge implicite
/// par défaut (qui varie selon la présence du manifeste comctl32 v6 et
/// n'est pas lisible via EM_GETMARGINS tant qu'elle n'a jamais été fixée
/// explicitement). Appliquée une fois dans apply_theme_visuals -- le
/// placeholder dessiné à la main (draw_placeholder) réutilise ensuite
/// EXACTEMENT cette même valeur (state.text_margin), garantissant que le
/// texte tapé et le placeholder démarrent au même endroit par construction,
/// plutôt que de deviner une marge ambiante.
const EM_SETMARGINS: u32 = 0xD3;
const EC_LEFTMARGIN: usize = 0x1;
const EC_RIGHTMARGIN: usize = 0x2;

/// Dessine "Type to search" à la main quand le champ est vide, avec la
/// police/couleur du THÈME courant -- remplace EM_SETCUEBANNER (voir le
/// commentaire sur placeholder_wide dans AppState) : ce message natif
/// dessine son texte avec une couleur interne à comctl32 qui ignore
/// SetTextColor, donc ne suit jamais un changement de thème contrairement
/// au reste de la barre.
unsafe fn draw_placeholder(hwnd: HWND, state: &AppState) {
    let hdc = GetDC(hwnd);
    if hdc.is_null() {
        return;
    }
    let mut rc = RECT::default();
    GetClientRect(hwnd, &mut rc);
    rc.left += state.text_margin;
    let old_font = SelectObject(hdc, state.font_search as _);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, state.theme.current.search_text);
    let len = (state.placeholder_wide.len() as i32 - 1).max(0);
    DrawTextW(hdc, state.placeholder_wide.as_ptr(), len, &mut rc, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
    SelectObject(hdc, old_font);
    ReleaseDC(hwnd, hdc);
}

/// Sous-classe des contrôles EDIT (recherche + horloge) : l'appli est
/// entièrement pilotée au clavier, la souris ne doit rien pouvoir y faire.
/// Bloquer les touches dans `handle_edit_keydown` ne
/// suffit pas : un vrai contrôle EDIT natif réagit aussi à la souris
/// (positionner le caret, sélectionner du texte, changer de curseur en
/// I-beam au survol) indépendamment du clavier -- donc tout message
/// souris est avalé ici (curseur forcé en flèche, clics/molette sans
/// effet) avant même d'atteindre la procédure d'origine ; tout le reste
/// (texte, focus, police...) lui est transmis normalement. WM_PAINT est
/// spécial : la procédure d'origine dessine d'abord le champ normalement,
/// puis le placeholder thémé est ajouté par-dessus si le champ est vide
/// (voir draw_placeholder) -- ne s'applique jamais à l'horloge, dont le
/// texte n'est jamais vide.
unsafe extern "system" fn edit_subclass_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK
        | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_MOUSEMOVE | WM_MOUSEWHEEL | WM_NCLBUTTONDOWN => 0,
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_ARROW));
            1
        }
        WM_PAINT => {
            let result = call_original_edit_proc(hwnd, msg, wparam, lparam);
            if GetWindowTextLengthW(hwnd) == 0 {
                if let Some(state) = get_state(GetParent(hwnd)) {
                    draw_placeholder(hwnd, state);
                }
            }
            result
        }
        _ => call_original_edit_proc(hwnd, msg, wparam, lparam),
    }
}

pub struct WindowHandles {
    pub main: HWND,
    pub edit: HWND,
}

/// Crée la fenêtre popup (cachée) et son contrôle de recherche. `apps` et
/// `theme` sont déplacés dans l'état de la fenêtre (GWLP_USERDATA) -- tout
/// leur cycle de vie est ensuite géré par la fenêtre elle-même (libéré au
/// WM_DESTROY).
pub fn create(apps: Vec<App>, mut theme_cfg: ThemeConfig, base_dir: PathBuf) -> Result<WindowHandles, String> {
    let class_name = to_wstring(WINDOW_CLASS_NAME);
    let window_name = to_wstring("MAGI Launcher");

    unsafe {
        let wc = simple_wndclass(
            class_name.as_ptr(),
            Some(wndproc),
            std::ptr::null_mut(),
            LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
        );
        if RegisterClassExW(&wc) == 0 {
            return Err(format!("RegisterClassExW a échoué (erreur {})", crate::win32::last_error()));
        }

        let themes_path = base_dir.join("themes.json");
        theme::load(&themes_path, &mut theme_cfg);

        let work = work_area_under_cursor();
        let geometry = compute_geometry(work, &theme_cfg);
        let g = geometry.window;

        let hwnd = CreateWindowExW(
            // WS_EX_COMPOSITED : demande à la DWM de composer la fenêtre
            // ET tous ses enfants (EDIT de recherche, horloge) via un
            // tampon hors écran commun -- sans ça, le double buffering
            // manuel de draw_scene ne couvre que le contenu dessiné par la
            // fenêtre elle-même, pas le cycle de peinture indépendant de
            // ses contrôles enfants natifs, qui pouvaient continuer à
            // clignoter (ex: l'horloge, rafraîchie chaque seconde) même
            // une fois la liste elle-même stabilisée.
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_COMPOSITED,
            class_name.as_ptr(),
            window_name.as_ptr(),
            // WS_CLIPCHILDREN : sans ce style, le rendu GDI de la fenêtre
            // (fill du fond de bordure sur tout le client, voir draw_scene)
            // dessine PAR-DESSUS les contrôles enfants (EDIT recherche +
            // horloge) au lieu d'être automatiquement découpé autour d'eux
            // -- l'horloge se faisait alors recouvrir par la couleur de
            // bordure à chaque InvalidateRect plein-écran (navigation
            // clavier), jusqu'à son prochain repaint naturel (tick
            // d'horloge suivant), ce qui se voyait comme un clignotement.
            WS_POPUP | WS_CLIPCHILDREN,
            g.left,
            g.top,
            rect_w(&g),
            rect_h(&g),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return Err(format!("CreateWindowExW a échoué (erreur {})", crate::win32::last_error()));
        }

        let (edit_rect, clock_rect) = search_control_rects(&geometry);

        let edit_class = to_wstring("EDIT");
        let edit_hwnd = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL as u32,
            edit_rect.left,
            edit_rect.top,
            rect_w(&edit_rect),
            rect_h(&edit_rect),
            hwnd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if edit_hwnd.is_null() {
            return Err(format!("création du contrôle EDIT échouée (erreur {})", crate::win32::last_error()));
        }

        // Horloge en lecture seule -- un vrai contrôle EDIT (pas STATIC) :
        // un STATIC ne centre pas verticalement son texte par défaut et
        // n'a pas les mêmes marges internes qu'un EDIT, ce qui le faisait
        // paraître décalé/désaligné par rapport au texte de recherche
        // juste à côté. Utiliser la MÊME classe de contrôle garantit un
        // rendu strictement identique (marges, centrage, police) sans
        // avoir à deviner/recopier les bons réglages à la main.
        // ES_READONLY empêche toute édition, ES_RIGHT aligne l'heure à
        // droite (avec la marge interne habituelle d'un EDIT) plutôt que
        // collée au bord gauche de son rectangle.
        let clock_hwnd = if theme_cfg.show_clock {
            CreateWindowExW(
                0,
                edit_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | (ES_READONLY | ES_RIGHT) as u32,
                clock_rect.left,
                clock_rect.top,
                rect_w(&clock_rect),
                rect_h(&clock_rect),
                hwnd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } else {
            std::ptr::null_mut()
        };

        // Sous-classement souris (voir edit_subclass_proc) -- capturé une
        // seule fois depuis edit_hwnd puisque les deux contrôles partagent
        // la même classe stock "EDIT", donc la même procédure d'origine.
        ORIGINAL_EDIT_WNDPROC
            .store(GetWindowLongPtrW(edit_hwnd, GWLP_WNDPROC), std::sync::atomic::Ordering::Relaxed);
        SetWindowLongPtrW(edit_hwnd, GWLP_WNDPROC, edit_subclass_proc as *const () as isize);
        if !clock_hwnd.is_null() {
            SetWindowLongPtrW(clock_hwnd, GWLP_WNDPROC, edit_subclass_proc as *const () as isize);
        }

        let notes_path = base_dir.join("notes.json");
        let restart_path = base_dir.join("restart.json");
        let notes = crate::core::json_list::load_notes(&notes_path);
        let restart_targets = crate::core::json_list::load_restart_list(&restart_path);
        let mut restart_supervisor = RestartSupervisor::new(restart_targets.clone());
        restart_supervisor.start();
        let emoji = crate::core::emoji::load(&base_dir.join("emoji-test.txt"));

        let mut state = Box::new(AppState {
            mode_items: apps.iter().map(|a| a.name.clone()).collect(),
            filtered: (0..apps.len()).collect(),
            apps,
            windows: Vec::new(),
            recycle_bin_items: Vec::new(),
            emoji,
            notes,
            notes_path,
            restart_targets,
            restart_path,
            restart_supervisor,
            auto_restart_enabled: true,
            mode: Mode::Normal,
            selected: 0,
            first_visible: 0,
            display: SearchDisplay::List,
            theme: theme_cfg,
            theme_picker_original: None,
            base_dir,
            themes_path,
            timer_deadline: None,
            timer_total_seconds: 0,
            bouncing: false,
            bounce_pos: (0.0, 0.0),
            bounce_vel: (0.0, 0.0),
            bounce_pre_geometry: None,
            bounce_pre_theme: None,
            rng: SimpleRng::new(),
            edit_hwnd,
            clock_hwnd,
            geometry,
            font_row: std::ptr::null_mut(),
            font_search: std::ptr::null_mut(),
            search_brush: std::ptr::null_mut(),
            placeholder_wide: Vec::new(),
            text_margin: 0,
            recycle_bin_cache: std::cell::Cell::new(None),
        });
        apply_theme_visuals(hwnd, &mut state);
        if !clock_hwnd.is_null() {
            SetWindowTextW(clock_hwnd, to_wstring(&crate::core::clock::format_now()).as_ptr());
            SetTimer(hwnd, CLOCK_TIMER_ID, 1000, None);
        }

        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        Ok(WindowHandles { main: hwnd, edit: edit_hwnd })
    }
}

/// Recalcule la géométrie (moniteur sous le curseur + réglages de thème
/// courants) et repositionne/redimensionne la fenêtre ET ses contrôles
/// enfants en conséquence -- partagé par show() (le moniteur sous le
/// curseur peut avoir changé depuis la dernière fois) et reload_config()
/// (la largeur/bordure/etc. peuvent avoir changé dans themes.json). Sans
/// ça, `state.geometry` se retrouverait décorrélée de la taille réelle de
/// la fenêtre à l'écran -- un bug réel trouvé en écrivant ce commentaire :
/// reload_config() recalculait déjà la géométrie mais ne l'appliquait
/// jamais à la fenêtre.
///
/// Limite connue : le contrôle STATIC de l'horloge est créé une seule
/// fois, à la création de la fenêtre, selon la valeur de `show_clock`
/// d'alors -- un Reload qui active/désactive `show_clock` dans
/// themes.json ne fait donc pas apparaître/disparaître l'horloge tant que
/// l'appli n'est pas relancée (cas marginal, edit à la main suivi d'un
/// Reload plutôt qu'un usage courant).
unsafe fn apply_geometry(hwnd: HWND, state: &mut AppState) {
    let work = work_area_under_cursor();
    state.geometry = compute_geometry(work, &state.theme);
    apply_theme_visuals(hwnd, state);
    let g = state.geometry.window;
    let (edit_rect, clock_rect) = search_control_rects(&state.geometry);
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
    if !state.clock_hwnd.is_null() {
        SetWindowPos(
            state.clock_hwnd,
            std::ptr::null_mut(),
            clock_rect.left,
            clock_rect.top,
            rect_w(&clock_rect),
            rect_h(&clock_rect),
            SWP_NOZORDER,
        );
        SetWindowTextW(state.clock_hwnd, to_wstring(&crate::core::clock::format_now()).as_ptr());
    }
}

pub unsafe fn show(hwnd: HWND) {
    let Some(state) = get_state(hwnd) else { return };
    if state.bouncing {
        stop_bounce(hwnd, state);
    }
    apply_geometry(hwnd, state);
    enter_mode(hwnd, state, Mode::Normal);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    SetActiveWindow(hwnd);
    SetFocus(state.edit_hwnd);
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

pub unsafe fn hide(hwnd: HWND) {
    ShowWindow(hwnd, SW_HIDE);
}

/// Instantané du style courant (couleurs, police, épaisseur de bordure) --
/// utilisé par main.rs pour habiller le menu contextuel du tray
/// (ui::popup_menu) aux couleurs du thème actif, sans lui exposer
/// `AppState` en entier.
pub unsafe fn menu_style(hwnd: HWND) -> Option<(super::popup_menu::MenuColors, String, i32)> {
    let state = get_state(hwnd)?;
    let t = &state.theme.current;
    Some((
        super::popup_menu::MenuColors {
            list_background: t.list_background,
            list_text: t.list_text,
            selected_background: t.selected_background,
            selected_text: t.selected_text,
            border: t.border,
        },
        theme::resolve_font_family(&state.theme),
        state.theme.border_width,
    ))
}

pub unsafe fn toggle(hwnd: HWND) {
    if IsWindowVisible(hwnd) != 0 {
        // Le raccourci global doit rester une des trois sorties du rebond
        // DVD (avec Échap et le clic souris, voir wndproc) -- sans ce
        // test, hide() masquait juste la fenêtre en laissant bouncing=true
        // et BOUNCE_TIMER_ID tourner en tâche de fond (repositionnement
        // recalculé à vide sur une fenêtre invisible) jusqu'au prochain
        // show(), qui s'en charge lui aussi mais bien plus tard.
        if let Some(state) = get_state(hwnd) {
            if state.bouncing {
                stop_bounce(hwnd, state);
            }
        }
        hide(hwnd);
    } else {
        show(hwnd);
    }
}

pub unsafe fn is_auto_restart_enabled(hwnd: HWND) -> bool {
    get_state(hwnd).map_or(true, |state| state.auto_restart_enabled)
}

/// Bascule le superviseur Auto-restart (voir "Disable/Enable Auto-restart"
/// dans le menu du tray) -- même principe que le hotkey : stop()/start()
/// plutôt que de vider `restart_targets`, pour ne rien perdre de la liste
/// surveillée pendant que c'est désactivé.
pub unsafe fn toggle_auto_restart(hwnd: HWND) {
    if let Some(state) = get_state(hwnd) {
        if state.auto_restart_enabled {
            state.restart_supervisor.stop();
        } else {
            state.restart_supervisor.start();
        }
        state.auto_restart_enabled = !state.auto_restart_enabled;
    }
}

/// Appelé par la boucle de messages AVANT TranslateMessage/DispatchMessage
/// pour tout WM_KEYDOWN destiné au contrôle EDIT -- `true` si la touche a
/// été traitée ici (ne doit PAS atteindre l'EDIT), `false` pour la laisser
/// suivre son chemin normal (saisie de texte, Ctrl+C...). Évite d'avoir à
/// sous-classer le contrôle EDIT pour un besoin aussi ciblé.
pub unsafe fn handle_edit_keydown(hwnd: HWND, vk: u16) -> bool {
    let Some(state) = get_state(hwnd) else { return false };

    // Pendant le rebond DVD, toute touche referme la popup -- même esprit
    // que l'original (Tab/Échap/hotkey seuls y mettaient fin), simplifié :
    // ici n'importe quelle touche suffit, le rebond n'a rien d'autre à
    // faire de la saisie clavier.
    if state.bouncing {
        stop_bounce(hwnd, state);
        hide(hwnd);
        return true;
    }

    let ctrl_down = (GetKeyState(VK_CONTROL as i32) as u16) & 0x8000 != 0;
    let shift_down = (GetKeyState(VK_SHIFT as i32) as u16) & 0x8000 != 0;

    // Ctrl+S/W/D/A sont de simples alias des flèches (déplacement/pagination
    // au clavier sans quitter le pavé de lettres) -- normalisés ici plutôt
    // que dupliqués en branches séparées du match ci-dessous.
    let vk = match (ctrl_down, vk) {
        (true, VK_S) => VK_DOWN,
        (true, VK_W) => VK_UP,
        (true, VK_D) => VK_RIGHT,
        (true, VK_A) => VK_LEFT,
        _ => vk,
    };

    let handled = match vk {
        VK_DOWN => {
            move_selection(hwnd, state, 1);
            true
        }
        VK_UP => {
            move_selection(hwnd, state, -1);
            true
        }
        VK_LEFT => {
            move_selection(hwnd, state, -(VISIBLE_ROWS as i32));
            true
        }
        VK_RIGHT => {
            move_selection(hwnd, state, VISIBLE_ROWS as i32);
            true
        }
        VK_RETURN => {
            if shift_down {
                reveal_or_edit(hwnd, state);
            } else {
                launch_selected(hwnd, state);
            }
            true
        }
        // Règle unique, peu importe le mode courant : Échap ramène TOUJOURS
        // au menu principal (et ferme le popup si on y est déjà) --
        // `exit_picker` fait déjà ça pour n'importe quel mode (annule aussi
        // une preview de thème non validée), rien de plus à faire ici.
        VK_ESCAPE => {
            if state.mode == Mode::Normal {
                hide(hwnd);
            } else {
                exit_picker(hwnd, state);
            }
            true
        }
        // Règle unique, peu importe le mode courant : Tab va TOUJOURS au
        // Window Switcher (les fenêtres actives) -- avant, Tab quittait
        // vers le menu principal depuis certains modes (Thème, Restart) et
        // ne faisait rien depuis d'autres (Timer), une règle par mode
        // plutôt qu'une seule cohérente partout.
        VK_TAB => {
            cancel_uncommitted_theme_preview(hwnd, state);
            enter_mode(hwnd, state, Mode::Window);
            true
        }
        VK_DELETE => {
            on_delete(hwnd, state, shift_down);
            true
        }
        _ => false,
    };
    if handled {
        InvalidateRect(hwnd, std::ptr::null(), 0);
    }
    handled
}

/// Suppr côté clavier appelle déjà `on_delete` via handle_edit_keydown ;
/// exposé séparément pour Retour arrière sur une recherche déjà vide, qui
/// doit sortir du picker actif plutôt que de ne rien faire -- vérifié par
/// l'appelant (le texte de l'EDIT n'a pas encore changé à ce stade,
/// puisqu'on est avant Translate/Dispatch).
pub unsafe fn handle_backspace_on_empty(hwnd: HWND) -> bool {
    let Some(state) = get_state(hwnd) else { return false };
    if state.mode == Mode::Normal {
        return false;
    }
    if !get_edit_text(state.edit_hwnd).is_empty() {
        return false;
    }
    exit_picker(hwnd, state);
    InvalidateRect(hwnd, std::ptr::null(), 0);
    true
}

/// Stress test/profilage mémoire de la fenêtre, sur un catalogue/thème
/// synthétiques dans un dossier temporaire (jamais le vrai apps.json/
/// notes.json/restart.json de l'utilisateur). Pilote la fenêtre en
/// appelant directement ses fonctions internes (jamais SendInput/
/// PostMessage -- pas de simulation d'événements clavier/souris au niveau
/// de l'OS), et ne l'affiche JAMAIS (pas d'appel à show()) : tout se passe
/// hors écran, sans jamais interrompre l'utilisateur.
///
/// SÉCURITÉ -- ce test ne doit JAMAIS :
/// - vider la vraie Corbeille (aucun sentinel `magi:*` dans le catalogue
///   factice, donc aucun chemin de code ne peut atteindre
///   core::recycle_bin::empty_async)
/// - activer/fermer/tuer une vraie fenêtre du bureau (Mode::Window
///   n'est jamais exercé au-delà de enter_mode/filtrage/move_selection --
///   jamais launch_selected ni on_delete pendant qu'on y est)
/// - lancer un vrai programme (les chemins du catalogue factice n'existent
///   pas sur le disque -- ShellExecuteExW échoue en silence, voir
///   core::launch)
#[cfg(test)]
mod stress_test {
    use super::*;
    use std::time::{Duration, Instant};

    fn sandbox_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_launcher_stress_{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fake_apps(n: usize) -> Vec<App> {
        let mut apps: Vec<App> = (0..n)
            .map(|i| App::new(format!("Stress App {i}"), format!("C:\\StressTest\\App{i}.exe"), None, false))
            .collect();
        // Chemin ultra long (au-delà de MAX_PATH) -- exerce la troncature/
        // ellipse du rendu (DrawTextW + DT_END_ELLIPSIS) sur une entrée
        // réaliste plutôt qu'un cas jamais atteignable en pratique.
        let huge_path = format!("C:\\{}\\App.exe", "StressSubfolder".repeat(60));
        apps.push(App::new("Stress App With A Very Long Path Indeed".to_string(), huge_path, None, false));
        apps
    }

    fn process_memory_kb() -> usize {
        unsafe {
            let mut counters = crate::win32::kernel32::PROCESS_MEMORY_COUNTERS::default();
            counters.cb = std::mem::size_of::<crate::win32::kernel32::PROCESS_MEMORY_COUNTERS>() as u32;
            crate::win32::kernel32::K32GetProcessMemoryInfo(
                crate::win32::kernel32::GetCurrentProcess(),
                &mut counters,
                counters.cb,
            );
            counters.WorkingSetSize / 1024
        }
    }

    fn gdi_object_count() -> u32 {
        unsafe {
            crate::win32::user32::GetGuiResources(
                crate::win32::kernel32::GetCurrentProcess(),
                crate::win32::user32::GR_GDIOBJECTS,
            )
        }
    }

    fn user_object_count() -> u32 {
        unsafe {
            crate::win32::user32::GetGuiResources(
                crate::win32::kernel32::GetCurrentProcess(),
                crate::win32::user32::GR_USEROBJECTS,
            )
        }
    }

    unsafe fn pump_messages(hwnd: HWND, ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(ms);
        let mut msg = crate::win32::MSG::default();
        while Instant::now() < deadline {
            while crate::win32::user32::PeekMessageW(&mut msg, hwnd, 0, 0, crate::win32::user32::PM_REMOVE) != 0 {
                crate::win32::user32::TranslateMessage(&msg);
                crate::win32::user32::DispatchMessageW(&msg);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn stress_gui_complet() {
        let sandbox = sandbox_dir("gui");

        // Copie le vrai themes.json (100+ thèmes) -- un vrai test du
        // sélecteur de thème plutôt qu'un unique thème de repli.
        let real_themes = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes.json");
        if real_themes.exists() {
            let _ = std::fs::copy(&real_themes, sandbox.join("themes.json"));
        }

        let apps = fake_apps(8);
        let handles = create(apps, ThemeConfig::default(), sandbox.clone()).expect("création fenêtre échouée");
        let hwnd = handles.main;

        // Requêtes volontairement adverses : division par zéro, expression
        // mal formée, hex court/trop long/majuscules, exposant énorme,
        // parenthèses profondément imbriquées, chaîne géante, unicode/
        // emoji, retours à la ligne/tabulations, aucune correspondance.
        let long_number = "9".repeat(400);
        let deep_parens = format!("{}1{}", "(".repeat(300), ")".repeat(300));
        let huge_query = "x".repeat(20_000);
        let queries: Vec<String> = vec![
            "stress".into(),
            "app".into(),
            "1".into(),
            "zzz-no-match".into(),
            "3+4*2".into(),
            "#3498db".into(),
            "".into(),
            "1/0".into(),
            "0/0".into(),
            "2+".into(),
            "#fff".into(),
            "#ABCDEF".into(),
            "#".into(),
            "#00000000000".into(),
            long_number,
            deep_parens,
            huge_query,
            "\u{1F600}\u{1F4A9} émoji tést \u{0000}".into(),
            "line1\nline2\ttab".into(),
            "999999999999999999999999999999*999999999999999999999999999999".into(),
        ];

        let iterations = 400;
        let mut baseline_mem = 0usize;
        let mut baseline_gdi = 0u32;

        for i in 0..iterations {
            let query = queries[i % queries.len()].clone();
            unsafe {
                let state = get_state(hwnd).expect("state manquant");
                set_edit_text(state.edit_hwnd, &query);
                refresh_filter(state);
            }

            unsafe {
                match i % 6 {
                    0 => {
                        // Thème : parcourt tout le catalogue réel avec
                        // preview live à chaque déplacement.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Theme);
                        let len = current_list_len(get_state(hwnd).unwrap());
                        for _ in 0..len.min(50) {
                            move_selection(hwnd, get_state(hwnd).unwrap(), 1);
                        }
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    1 => {
                        // Window Switcher : LECTURE SEULE (jamais
                        // launch_selected/on_delete ici -- activerait/
                        // fermerait une vraie fenêtre du bureau).
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Window);
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, "a");
                        refresh_filter(get_state(hwnd).unwrap());
                        move_selection(hwnd, get_state(hwnd).unwrap(), 3);
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, "");
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    2 => {
                        // Timer -- armé pour de vrai (rebond DVD réel) de
                        // temps en temps seulement, fenêtre jamais montrée.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Timer);
                        if i % 150 == 2 {
                            arm_timer(hwnd, get_state(hwnd).unwrap(), 1);
                            pump_messages(hwnd, 1300);
                            let state = get_state(hwnd).unwrap();
                            if state.bouncing {
                                stop_bounce(hwnd, state);
                            } else {
                                cancel_timer(hwnd, state);
                            }
                        } else {
                            cancel_timer(hwnd, get_state(hwnd).unwrap());
                        }
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    3 => {
                        // Notes : ajout PUIS suppression réels (persistance
                        // sur notes.json dans le sandbox), assertions
                        // dédiées.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Notes);
                        let note_text = format!("stress-note-{i}");
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &note_text);
                        refresh_filter(get_state(hwnd).unwrap());
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        assert!(
                            get_state(hwnd).unwrap().notes.contains(&note_text),
                            "note '{note_text}' jamais ajoutée"
                        );
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &note_text);
                        refresh_filter(get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        assert!(
                            !get_state(hwnd).unwrap().notes.contains(&note_text),
                            "note '{note_text}' jamais supprimée"
                        );
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    4 => {
                        // Auto-restart : même principe que Notes.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Restart);
                        let target = format!("C:\\StressTest\\AutoRestart{i}.exe");
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &target);
                        refresh_filter(get_state(hwnd).unwrap());
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        assert!(
                            get_state(hwnd).unwrap().restart_targets.contains(&target),
                            "cible '{target}' jamais ajoutée"
                        );
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &target);
                        refresh_filter(get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        assert!(
                            !get_state(hwnd).unwrap().restart_targets.contains(&target),
                            "cible '{target}' jamais supprimée"
                        );
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    _ => {
                        // Corbeille : lecture seule -- Entrée/Suppr sont
                        // des no-ops voulus dans ce mode (voir
                        // launch_selected/on_delete), donc sans risque.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::RecycleBin);
                        move_selection(hwnd, get_state(hwnd).unwrap(), 2);
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                }
            }

            // Navigation clavier universelle via le VRAI point d'entrée
            // (handle_edit_keydown), pas un appel direct à enter_mode --
            // exerce la normalisation Ctrl+S/W/D/A et les règles Tab/Échap
            // telles qu'un vrai WM_KEYDOWN les déclencherait.
            unsafe {
                handle_edit_keydown(hwnd, VK_DOWN);
                handle_edit_keydown(hwnd, VK_UP);
                handle_edit_keydown(hwnd, VK_TAB); // -> Window Switcher
                handle_edit_keydown(hwnd, VK_ESCAPE); // -> retour Normal
            }

            // Perte de focus pendant une preview de thème non validée --
            // exerce directement le correctif WM_ACTIVATE.
            unsafe {
                if i % 15 == 0 {
                    let state = get_state(hwnd).unwrap();
                    enter_mode(hwnd, state, Mode::Theme);
                    move_selection(hwnd, get_state(hwnd).unwrap(), 1);
                    wndproc(hwnd, WM_ACTIVATE, WA_INACTIVE as WPARAM, 0);
                }

                // Lancement "normal" d'une appli factice -- chemin
                // inexistant, ShellExecuteExW échoue en silence
                // (SEE_MASK_FLAG_NO_UI), aucun programme ne démarre
                // réellement.
                if i % 7 == 0 {
                    let state = get_state(hwnd).unwrap();
                    enter_mode(hwnd, state, Mode::Normal);
                    set_edit_text(state.edit_hwnd, &format!("Stress App {}", i % 8));
                    refresh_filter(get_state(hwnd).unwrap());
                    launch_selected(hwnd, get_state(hwnd).unwrap());
                }

                // Reload en alternant show_clock (recharge aussi le vrai
                // themes.json copié dans le sandbox).
                if i % 25 == 0 {
                    reload_config(hwnd, get_state(hwnd).unwrap());
                }
            }

            if i == 50 {
                baseline_mem = process_memory_kb();
                baseline_gdi = gdi_object_count();
            }
            if i > 50 && i % 100 == 0 {
                eprintln!(
                    "iter {i}: mem={}KB (baseline {baseline_mem}KB) gdi={} user={}",
                    process_memory_kb(),
                    gdi_object_count(),
                    user_object_count()
                );
            }
        }

        let final_mem = process_memory_kb();
        let final_gdi = gdi_object_count();
        let final_user = user_object_count();
        eprintln!(
            "=== final : mem={final_mem}KB (baseline {baseline_mem}KB)  gdi={final_gdi} (baseline {baseline_gdi})  user={final_user} ==="
        );

        unsafe {
            crate::win32::user32::DestroyWindow(hwnd);
        }
        let _ = std::fs::remove_dir_all(&sandbox);

        // Un vrai grossissement linéaire du nombre d'objets GDI/USER avec
        // les itérations trahirait une fuite de handle (police/pinceau
        // jamais détruits) -- marge généreuse (x3) pour tolérer la
        // variance normale sans être aveugle à une vraie fuite.
        assert!(
            (final_gdi as u64) <= (baseline_gdi.max(20) as u64) * 3,
            "fuite d'objets GDI suspectée : {baseline_gdi} -> {final_gdi}"
        );
    }

    #[test]
    fn rejette_des_reglages_de_theme_extremes_sans_planter() {
        let work = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        let sandbox = sandbox_dir("theme_extreme");
        let cases = [
            r##"{"theme":"t","window_width_fraction":1e300,"border":1000000000,"themes":{"t":{"search_background":"#000000","search_text":"#ffffff","list_background":"#000000","list_text":"#ffffff","selected_background":"#000000","selected_text":"#ffffff","border":"#000000"}}}"##,
            r##"{"theme":"t","window_width_fraction":-1,"border":-2147483648,"themes":{"t":{"search_background":"#000000","search_text":"#ffffff","list_background":"#000000","list_text":"#ffffff","selected_background":"#000000","selected_text":"#ffffff","border":"#000000"}}}"##,
            r##"{"theme":"t","window_width_fraction":0,"border":2147483647,"themes":{"t":{"search_background":"#000000","search_text":"#ffffff","list_background":"#000000","list_text":"#ffffff","selected_background":"#000000","selected_text":"#ffffff","border":"#000000"}}}"##,
        ];
        for (i, case) in cases.iter().enumerate() {
            let path = sandbox.join(format!("themes_{i}.json"));
            std::fs::write(&path, case).unwrap();
            let mut cfg = ThemeConfig::default();
            theme::load(&path, &mut cfg);
            let g = compute_geometry(work, &cfg);
            eprintln!(
                "cas {i} -> fraction={} border={} -> window left={} top={} right={} bottom={}",
                cfg.window_width_fraction, cfg.border_width, g.window.left, g.window.top, g.window.right,
                g.window.bottom
            );
        }
        let _ = std::fs::remove_dir_all(&sandbox);
    }
}
