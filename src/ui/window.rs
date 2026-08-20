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
use crate::core::clipboard_history::ClipboardHistory;
use crate::core::disk_ejector::EjectableDrive;
use crate::core::emoji::EmojiData;
use crate::core::models::App;
use crate::core::recycle_bin::RecycleBinItem;
use crate::core::search::{match_rank_multi, normalize};
use crate::core::supervisor::RestartSupervisor;
use crate::core::windows::WindowEntry;
use crate::win32::gdi32::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, EndPaint, FillRect, GetMonitorInfoW, InvalidateRect, MonitorFromPoint, RedrawWindow,
    SelectObject, SetBkColor, SetBkMode, SetTextColor, HBITMAP, HDC, HFONT, MONITORINFO, OPAQUE, PAINTSTRUCT,
    TRANSPARENT, DT_END_ELLIPSIS, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, RDW_ALLCHILDREN, RDW_ERASE,
    RDW_INVALIDATE, RDW_UPDATENOW, SRCCOPY,
};
use crate::win32::kernel32::{AttachThreadInput, GetCurrentThreadId};
use crate::win32::user32::{
    CallWindowProcW, CreateWindowExW, DefWindowProcW, GetCaretPos, GetClientRect, GetCursorPos, GetForegroundWindow,
    GetKeyState, GetParent, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    HideCaret, IsWindowVisible, KillTimer, LoadCursorW, SetCaretPos, ShowCaret,
    RegisterClassExW, SendMessageW, SetActiveWindow, SetCursor, SetFocus, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, HBRUSH, WNDPROC,
    EN_CHANGE, ES_AUTOHSCROLL, GWLP_USERDATA, GWLP_WNDPROC, HWND_TOPMOST, IDC_ARROW,
    MONITOR_DEFAULTTONEAREST, SWP_NOZORDER, SW_HIDE, SW_SHOW, VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8,
    VK_9, VK_A, VK_CONTROL, VK_D, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_OEM_MINUS, VK_OEM_PLUS, VK_RETURN,
    VK_RIGHT, VK_S, VK_SHIFT, VK_TAB, VK_UP, VK_W, WA_INACTIVE, WM_ACTIVATE, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCLBUTTONDOWN, WM_PAINT,
    WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFONT, WM_TIMER, WS_CHILD, WS_CLIPCHILDREN,
    WS_EX_COMPOSITED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    AddClipboardFormatListener, RemoveClipboardFormatListener, WM_CLIPBOARDUPDATE,
};
use crate::win32::{
    from_wstring, get_clipboard_text, set_clipboard_text, to_wstring, HWND, LPARAM, LRESULT, POINT, RECT, UINT,
    WPARAM,
};
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
/// Ratio hauteur/largeur de la fenêtre -- 16:9 strict, indépendant du
/// contenu.
const HEIGHT_RATIO: f64 = 9.0 / 16.0;
/// Taille de police plancher (voir apply_theme_visuals) -- en-dessous, un
/// EDIT natif ne centre plus fiablement son texte.
const MIN_FONT_PX: i32 = 9;
const WINDOW_CLASS_NAME: &str = "MAGILauncherPopupClass";

const CLOCK_TIMER_ID: usize = 1;
const COUNTDOWN_TIMER_ID: usize = 2;
const FIRE_TIMER_ID: usize = 3;
const BOUNCE_TIMER_ID: usize = 4;
const RECYCLE_BIN_POLL_TIMER_ID: usize = 5;
/// Cadence de sondage du résultat du scan Corbeille en arrière-plan (voir
/// rebuild_recyclebin_items) -- assez court pour que la liste apparaisse
/// sans délai perceptible une fois le scan terminé, sans pomper le thread
/// UI pour rien entre-temps.
const RECYCLE_BIN_POLL_INTERVAL_MS: u32 = 80;
const BOUNCE_INTERVAL_MS: u32 = 16;
/// px/tick à ~60fps (16ms). Valeur fixe en pixels, pas une fraction de la
/// largeur d'écran : la vitesse perçue doit rester la même quelle que soit
/// la taille du moniteur.
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
const SENTINEL_COPY_HISTORY: &str = "magi:copy-history";
const SENTINEL_EJECT: &str = "magi:eject";

/// Remplacent `theme.placeholder_text` dans les modes qui attendent une
/// saisie plutôt qu'une recherche. Les modes réellement de recherche
/// (Window Switcher, Thème, Corbeille, Emoji) gardent le placeholder du
/// thème.
const TIMER_PLACEHOLDER: &str = "Type a duration (5m, 90s, 1h...)";
const NOTES_PLACEHOLDER: &str = "Type a note...";
const RESTART_PLACEHOLDER: &str = "Type a target to watch...";

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
    CopyHistory,
    Eject,
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

/// Marge générale unique que TOUT texte respecte (lignes de résultat,
/// placeholder, horloge) -- tout le reste (blocs, rects, contrôles) va bord
/// à bord ; seule cette marge insère du vide avant le texte. Point de
/// calcul unique : un ratio dupliqué ailleurs divergerait au premier
/// arrondi.
fn text_margin_px(row_h: i32) -> i32 {
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
fn search_control_rect(geometry: &Geometry) -> RECT {
    let control_h = rect_h(&geometry.rows[0]);
    centered_control_rect(&geometry.search, control_h)
}

/// Générateur pseudo-aléatoire (xorshift64*) -- direction initiale du
/// rebond DVD et choix du thème à chaque collision. Aucun besoin
/// cryptographique, donc pas de dépendance externe.
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

    /// Flottant uniforme dans [0, 1) -- angle initial du rebond DVD.
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
    /// Contenu de la Corbeille (voir Mode::RecycleBin) -- `mode_items` n'en
    /// dérive que les noms ; gardé à part, comme `windows`, pour retrouver
    /// le chemin `$I`/`$R` réel d'un élément sélectionné.
    recycle_bin_items: Vec<RecycleBinItem>,
    /// `Some` tant qu'un scan Corbeille lancé sur un thread dédié (voir
    /// rebuild_recyclebin_items) n'a pas rendu son résultat -- sondé par
    /// RECYCLE_BIN_POLL_TIMER_ID. `list_items()` énumère TOUS les lecteurs
    /// sur disque : l'appeler sur le thread UI gèlerait le lanceur le temps
    /// du scan, même raison que empty_async() côté vidage.
    recycle_bin_pending: Option<std::sync::mpsc::Receiver<Vec<RecycleBinItem>>>,
    /// Contenu du mode Eject -- même rôle que `windows` pour le Window
    /// Switcher : `mode_items` n'en dérive que le libellé, la lettre de
    /// lecteur réelle reste ici (voir launch_selected).
    eject_drives: Vec<EjectableDrive>,
    /// `None` si emoji-test.txt est absent/illisible à côté de l'exe (voir
    /// core::emoji::load) -- le mode Emoji reste alors inatteignable (voir
    /// launch_selected_normal) plutôt que d'ouvrir un picker vide.
    emoji: Option<EmojiData>,
    notes: Vec<String>,
    notes_path: PathBuf,
    restart_targets: Vec<String>,
    restart_path: PathBuf,
    restart_supervisor: RestartSupervisor,
    /// Reflète si `restart_supervisor` tourne -- exposé au tray (voir
    /// "Disable Auto-restart").
    auto_restart_enabled: bool,

    /// RAM-only, jamais écrit sur disque -- voir core::clipboard_history
    /// pour le détail de sécurité (VirtualLock + effacement à zéro, sans
    /// chiffrement). Alimenté par WM_CLIPBOARDUPDATE tant que
    /// `copy_history_enabled` est actif.
    copy_history: ClipboardHistory,
    /// Reflète si le listener presse-papier (AddClipboardFormatListener)
    /// est enregistré -- opt-in, `false` par défaut (voir
    /// core::config::Config::copy_history_enabled).
    copy_history_enabled: bool,
    /// Positionné juste avant un `set_clipboard_text` émis par le lanceur
    /// lui-même (re-copie d'une entrée de l'historique) et consommé par le
    /// prochain WM_CLIPBOARDUPDATE, qui est alors ignoré : sans ça, la
    /// re-copie réinjecterait un doublon en tête de l'historique.
    suppress_next_clipboard_capture: bool,

    mode: Mode,
    /// Libellés actuellement filtrables/affichables -- reflète `apps` en
    /// mode Normal, `windows`/`notes`/`restart_targets`/les noms de thème
    /// dans les autres modes. Un seul chemin de filtrage/rendu pour tous
    /// les modes (voir le commentaire en tête de fichier).
    mode_items: Vec<String>,
    /// `(dernier mode_items vu, sa version normalisée)` -- accents repliés
    /// et minuscules, voir `normalized_mode_items`. `fuzzy_filter` tourne à
    /// chaque frappe alors que `mode_items` ne change qu'à l'entrée d'un
    /// mode ou après un ajout/suppression : renormaliser plusieurs milliers
    /// d'entrées (catalogue Emoji, une String allouée chacune) à chaque
    /// frappe serait du travail perdu. Invalidation par comparaison
    /// d'égalité, donc jamais désynchronisé -- aucun compteur à incrémenter
    /// sur les nombreux sites qui réaffectent `mode_items`.
    mode_items_cache: (Vec<String>, Vec<String>),
    filtered: Vec<usize>,
    selected: usize,
    first_visible: usize,
    display: SearchDisplay,

    theme: ThemeConfig,
    theme_picker_original: Option<String>,
    /// `false` si themes.json est absent/invalide/sans thème exploitable
    /// (voir theme::load) : `theme.current` retombe sur le thème de secours
    /// codé en dur. Même principe que `emoji` pour emoji-test.txt -- rendu
    /// explicite dans row_label ("Theme: missing themes.json") et bloque
    /// l'entrée dans le sélecteur, plutôt que de masquer le problème
    /// derrière le thème de secours.
    themes_json_present: bool,
    base_dir: PathBuf,
    themes_path: PathBuf,

    // Timer + rebond DVD
    timer_deadline: Option<Instant>,
    bouncing: bool,
    bounce_pos: (f64, f64),
    bounce_vel: (f64, f64),
    bounce_pre_geometry: Option<Geometry>,
    bounce_pre_theme: Option<String>,
    rng: SimpleRng,

    edit_hwnd: HWND,
    geometry: Geometry,
    font_row: HFONT,
    font_search: HFONT,
    /// Famille/taille (px) actuellement appliquées à font_row/font_search --
    /// comparées à chaque appel de apply_theme_visuals() pour ne recréer
    /// les polices que si l'une des deux a changé, et pas à chaque
    /// changement de thème couleur.
    applied_font_family: String,
    applied_font_px: i32,
    search_brush: HBRUSH,
    /// Pinceaux de thème mis en cache -- créés une fois par changement de
    /// thème (apply_theme_visuals), jamais dans draw_scene. Sans ce cache,
    /// chaque case de la liste crée puis détruit son propre pinceau à
    /// CHAQUE repaint (jusqu'à ~14 paires CreateSolidBrush/DeleteObject par
    /// frame), coût payé en plein sur un défilement continu.
    list_bg_brush: HBRUSH,
    selected_bg_brush: HBRUSH,
    border_brush: HBRUSH,
    /// Tampon hors-écran pour draw_scene (voir WM_PAINT). draw_scene peint
    /// en PLUSIEURS FillRect/DrawTextW successifs (bordure plein-fenêtre
    /// d'abord, puis recherche/séparateur/lignes par-dessus), jamais en une
    /// opération atomique : dessiner ces étapes directement sur le DC de la
    /// fenêtre ouvre une fenêtre de temps pendant laquelle la DWM peut
    /// présenter une frame intermédiaire -- un flash couleur de bordure
    /// d'une frame, d'autant plus probable que la fenêtre/police est
    /// grande (plus de travail GDI entre les étapes). `WS_EX_COMPOSITED` n'y
    /// suffit pas : il synchronise le cycle de peinture des CONTRÔLES
    /// ENFANTS avec celui de la fenêtre, sans rendre nos propres appels GDI
    /// atomiques. Tout draw_scene est donc peint ici, puis présenté par un
    /// seul BitBlt -- atomique par construction, quel que soit le temps de
    /// remplissage. Recréé uniquement quand la taille de fenêtre change
    /// (même garde-fou que font_row/les pinceaux), jamais par frame.
    mem_dc: HDC,
    mem_bitmap: HBITMAP,
    /// Dimensions pour lesquelles mem_dc/mem_bitmap ont été créés -- pour ne
    /// les recréer QUE si la taille a réellement changé (voir
    /// ensure_scene_buffer), même principe que applied_font_family/px.
    mem_buffer_size: (i32, i32),
    /// Texte d'invite, dessiné à la main dans edit_subclass_proc -- gardé
    /// en UTF-16 prêt pour DrawTextW plutôt que reconverti à chaque repaint.
    placeholder_wide: Vec<u16>,
    /// Marge fixée sur edit_hwnd via EM_SETMARGINS (voir
    /// apply_theme_visuals), réutilisée telle quelle par draw_placeholder et
    /// par l'horloge (draw_clock_text) : même marge générale que tout texte
    /// respecte (voir text_margin_px).
    text_margin: i32,
    /// Cache de `recycle_bin::query()` (compte + taille). Un Cell, donc
    /// modifiable à travers un &AppState partagé (voir row_label) :
    /// SHQueryRecycleBinW touche le disque et peut prendre plusieurs
    /// dizaines de ms, assez pour bloquer la pompe de messages une frame à
    /// chaque frappe si la ligne Corbeille est visible.
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

/// Vide la Corbeille et rafraîchit le cache count/taille -- partagé par les
/// trois sites qui déclenchent un vidage complet (Maj+Entrée/Suppr sur la
/// ligne du menu principal, Maj+Suppr depuis la vue de consultation). Ne
/// ferme pas le lanceur : même convention que Suppr sur le Timer, une
/// action ponctuelle n'est pas une raison de quitter.
unsafe fn empty_recycle_bin(hwnd: HWND, state: &AppState) {
    crate::core::recycle_bin::empty_async();
    // Cache forcé à (0, 0) plutôt qu'invalidé : le vidage est asynchrone,
    // donc un `None` ferait re-interroger la Corbeille au tout prochain
    // repaint -- course quasi toujours perdue contre le thread de vidage à
    // peine lancé, dont le résultat "pas encore vide" resterait ensuite
    // figé pendant tout RECYCLE_BIN_CACHE_TTL. (0, 0) reflète l'intention
    // de l'action ; le TTL corrige de lui-même si le vidage échoue.
    state.recycle_bin_cache.set(Some((Instant::now(), 0, 0)));
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

unsafe fn get_state<'a>(hwnd: HWND) -> Option<&'a mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    ptr.as_mut()
}

/// Recalcule `placeholder_wide` selon le mode courant. Appelée par
/// apply_theme_visuals (un changement de thème ne doit pas écraser un
/// placeholder de mode par le générique) ET par enter_mode (un changement
/// de mode seul doit aussi mettre à jour le texte affiché).
fn refresh_placeholder(state: &mut AppState) {
    let text = match state.mode {
        Mode::Timer => TIMER_PLACEHOLDER,
        Mode::Notes => NOTES_PLACEHOLDER,
        Mode::Restart => RESTART_PLACEHOLDER,
        _ => state.theme.placeholder_text.as_str(),
    };
    state.placeholder_wide = to_wstring(text);
}

/// Reconstruit le look de la fenêtre (polices, pinceaux, marges,
/// placeholder) à partir de `state.theme.current` -- appelée à la création
/// et à chaque changement de thème. Deux polices distinctes : la recherche
/// reste légèrement plus grande que les lignes de résultat.
unsafe fn apply_theme_visuals(state: &mut AppState) {
    let row_h = rect_h(&state.geometry.rows[0]);
    let family = theme::resolve_font_family(&state.theme);
    // Plancher de lisibilité : sans lui, une fenêtre réduite au minimum
    // descend à 3-4px de police, taille à laquelle un EDIT natif ne centre
    // plus fiablement son texte.
    let row_font_px = ((row_h as f64 * 0.6) as i32).max(MIN_FONT_PX);

    // Ne recrée les polices QUE si la famille/taille a changé. `font_family`
    // est un réglage GLOBAL de themes.json, pas par thème couleur : sur tous
    // les appels où seules les COULEURS changent (bascule de thème pendant
    // le rebond DVD, preview en direct à chaque flèche du sélecteur), la
    // police demandée est identique à l'appel précédent et la recréer
    // coûterait deux DeleteObject + CreateFontIndirectW par pas de
    // défilement.
    let fonts_need_update =
        state.font_row.is_null() || family != state.applied_font_family || row_font_px != state.applied_font_px;
    if fonts_need_update {
        if !state.font_row.is_null() {
            DeleteObject(state.font_row as _);
        }
        if !state.font_search.is_null() {
            DeleteObject(state.font_search as _);
        }
        state.font_row = make_font(&family, row_font_px);
        // 1.2x la police des lignes, pas 2x : le bloc de recherche fait deux
        // fois la hauteur d'une ligne, mais le CONTRÔLE EDIT garde une
        // hauteur de ligne normale et est centré dedans (voir
        // centered_control_rect). L'horloge partage cette police, sélectionnée
        // à la main dans draw_scene -- pas de WM_SETFONT séparé pour elle,
        // ce n'est pas un contrôle réel.
        state.font_search = make_font(&family, (row_font_px as f64 * 1.2) as i32);
        state.applied_font_family = family;
        state.applied_font_px = row_font_px;
        SendMessageW(state.edit_hwnd, WM_SETFONT, state.font_search as usize, 1);
    }

    if !state.search_brush.is_null() {
        DeleteObject(state.search_brush as _);
    }
    state.search_brush = CreateSolidBrush(state.theme.current.search_background);
    // Pinceaux réutilisés tels quels par CHAQUE case de draw_scene --
    // recréés seulement ici (changement de thème réel ou preview live),
    // jamais dans la boucle de rendu elle-même.
    if !state.list_bg_brush.is_null() {
        DeleteObject(state.list_bg_brush as _);
    }
    state.list_bg_brush = CreateSolidBrush(state.theme.current.list_background);
    if !state.selected_bg_brush.is_null() {
        DeleteObject(state.selected_bg_brush as _);
    }
    state.selected_bg_brush = CreateSolidBrush(state.theme.current.selected_background);
    if !state.border_brush.is_null() {
        DeleteObject(state.border_brush as _);
    }
    state.border_brush = CreateSolidBrush(state.theme.current.border);

    // Marge interne fixée explicitement plutôt que de dépendre de la marge
    // implicite de l'EDIT (non lisible tant qu'elle n'a jamais été posée,
    // voir EM_SETMARGINS). Même fonction text_margin_px que draw_row_text,
    // pas un ratio recopié : une seule marge partagée par construction.
    // Peu coûteux, donc recalculé à chaque appel sans garde-fou.
    state.text_margin = text_margin_px(row_h);
    let margins_lparam = state.text_margin as isize | ((state.text_margin as isize) << 16);
    SendMessageW(state.edit_hwnd, EM_SETMARGINS, EC_LEFTMARGIN | EC_RIGHTMARGIN, margins_lparam);
    // Pas de EM_SETCUEBANNER pour le texte d'invite : ce message peint son
    // texte avec une couleur interne à comctl32 qui ignore SetTextColor, et
    // ne suit donc jamais la couleur `search_text` du thème.
    // `placeholder_wide` est peint à la main dans edit_subclass_proc.
    refresh_placeholder(state);

    // Le contrôle EDIT a son propre cycle de peinture, indépendant de celui
    // de la fenêtre parente. Un InvalidateRect ne fait que MARQUER la zone
    // sale et son WM_PAINT est le message de plus basse priorité de la file :
    // tant que d'autres messages arrivent derrière (frappe, changement de
    // mode enchaîné), l'ancienne couleur reste affichée bien après le
    // changement d'état interne. RDW_UPDATENOW force ce WM_PAINT ici.
    RedrawWindow(state.edit_hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASE);
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

/// Lance le scan de la Corbeille -- une fois à l'entrée du mode, pas à
/// chaque frappe. `list_items()` énumère TOUS les lecteurs sur disque (I/O
/// potentiellement lente), donc déporté sur un thread dédié comme
/// `recycle_bin::empty_async()` : le résultat arrive via
/// `recycle_bin_pending`, sondé par RECYCLE_BIN_POLL_TIMER_ID. La liste
/// démarre vide et se peuple au retour du scan, sans bloquer l'affichage.
unsafe fn rebuild_recyclebin_items(hwnd: HWND, state: &mut AppState) {
    state.recycle_bin_items.clear();
    state.mode_items.clear();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(crate::core::recycle_bin::list_items());
    });
    state.recycle_bin_pending = Some(rx);
    SetTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID, RECYCLE_BIN_POLL_INTERVAL_MS, None);
}

/// Sondé par WM_TIMER(RECYCLE_BIN_POLL_TIMER_ID) -- récupère le résultat du
/// scan dès qu'il est prêt. Met toujours à jour `recycle_bin_items` (lu
/// aussi hors de ce mode, voir row_label/on_delete), mais ne touche
/// `mode_items`/le filtre/le rendu que si on est ENCORE dans
/// Mode::RecycleBin : sans cette garde, un scan qui revient après une
/// sortie du mode écraserait la liste du mode devenu actif.
unsafe fn poll_recycle_bin(hwnd: HWND, state: &mut AppState) {
    let Some(rx) = &state.recycle_bin_pending else { return };
    match rx.try_recv() {
        Ok(items) => {
            state.recycle_bin_items = items;
            state.recycle_bin_pending = None;
            KillTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID);
            if state.mode == Mode::RecycleBin {
                sync_recyclebin_mode_items(state);
                refresh_filter(state);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            state.recycle_bin_pending = None;
            KillTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID);
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {}
    }
}

/// Libellés du mode Corbeille à partir de `recycle_bin_items` -- partagé
/// par le retour de scan (poll_recycle_bin) et la suppression optimiste
/// d'un élément (on_delete).
fn sync_recyclebin_mode_items(state: &mut AppState) {
    state.mode_items = state.recycle_bin_items.iter().map(|item| item.name.clone()).collect();
}

/// Libellés du Window Switcher à partir de `windows` -- partagé par la
/// (ré)énumération et la suppression optimiste d'une entrée (on_delete).
fn sync_window_mode_items(state: &mut AppState) {
    state.mode_items = state.windows.iter().map(|w| w.title.clone()).collect();
}

fn rebuild_window_items(state: &mut AppState) {
    state.windows = crate::core::windows::list_windows();
    sync_window_mode_items(state);
}

/// Synchrone, contrairement à rebuild_recyclebin_items :
/// `list_ejectable_drives` ne touche qu'une poignée de lettres de lecteur
/// avec quelques IOCTL chacune, rien qui justifie un thread dédié.
fn rebuild_eject_items(state: &mut AppState) {
    state.eject_drives = crate::core::disk_ejector::list_ejectable_drives();
    state.mode_items = state.eject_drives.iter().map(eject_drive_label).collect();
}

fn eject_drive_label(drive: &EjectableDrive) -> String {
    if drive.label.is_empty() {
        format!("{}:  Removable Disk", drive.letter)
    } else {
        format!("{}:  {}", drive.letter, drive.label)
    }
}

/// Partagé par Entrée (`force = false`) et Maj+Suppr (`force = true`) --
/// seul `force` change, la mise à jour de la liste locale en cas de succès
/// est identique. N'invalide pas le rendu (même convention que les branches
/// de on_delete) : à l'appelant de le faire une fois la réponse connue.
fn eject_selected(state: &mut AppState, idx: usize, force: bool) -> bool {
    let Some(drive) = state.eject_drives.get(idx).cloned() else { return false };
    if !crate::core::disk_ejector::eject_drive(drive.letter, force) {
        return false;
    }
    state.eject_drives.remove(idx);
    state.mode_items = state.eject_drives.iter().map(eject_drive_label).collect();
    refresh_filter(state);
    true
}

fn rebuild_theme_items(state: &mut AppState) {
    state.mode_items = theme::list_theme_names(&state.theme);
}

fn rebuild_emoji_items(state: &mut AppState) {
    state.mode_items =
        state.emoji.as_ref().map(|d| d.entries.iter().map(|e| e.name.clone()).collect()).unwrap_or_default();
}

/// Contenu de l'historique presse-papier -- déjà entièrement en RAM
/// (core::clipboard_history), aucune I/O à déporter ici.
fn rebuild_copy_history_items(state: &mut AppState) {
    state.mode_items = (0..state.copy_history.len()).filter_map(|i| state.copy_history.get(i).map(str::to_string)).collect();
}

/// "<nom>: <valeur>" -- moule commun aux branches de `row_label`, qui ne
/// diffèrent que par la valeur affichée.
fn suffixed(name: &str, value: impl std::fmt::Display) -> String {
    format!("{name}: {value}")
}

/// Libellé affiché pour l'entrée `idx` du mode courant -- dérive de
/// `mode_items`, sauf pour les entrées du mode Normal qui affichent une
/// valeur recalculée à chaque rendu (compte de la Corbeille, compte à
/// rebours du Timer, note la plus récente...).
fn row_label(state: &AppState, idx: usize) -> String {
    if state.mode == Mode::Normal {
        if let Some(app) = state.apps.get(idx) {
            // Suffixe "<nom>: ..." même à l'état vide : une entrée qui
            // affiche tantôt son nom brut, tantôt "nom: état", laisserait
            // croire qu'elle ne fait jamais rien. Seule la Corbeille déroge
            // à la règle -- à l'état vide, "Empty Recycle Bin" seul reste
            // plus clair qu'un "Empty Recycle Bin: " sans valeur.
            match app.path.as_str() {
                SENTINEL_EMPTY_RECYCLE_BIN => {
                    let (count, size) = recycle_bin_cached(state);
                    return if count > 0 {
                        format!("{}: {} items, {:.1} MB", app.name, count, size as f64 / 1_048_576.0)
                    } else {
                        app.name.clone()
                    };
                }
                SENTINEL_TIMER => {
                    let value = match state.timer_deadline {
                        Some(deadline) => {
                            let remaining = (deadline - Instant::now()).as_secs() as i64;
                            crate::core::timer::format_remaining(remaining)
                        }
                        None => "--:--".to_string(),
                    };
                    return suffixed(&app.name, value);
                }
                // La plus récente en premier (voir launch_selected :
                // insert(0, ..)), donc notes[0] est bien la dernière ajoutée.
                SENTINEL_NOTES => {
                    return match state.notes.first() {
                        Some(latest) => suffixed(&app.name, latest),
                        None => format!("{}:", app.name),
                    };
                }
                SENTINEL_RESTART => return suffixed(&app.name, state.restart_targets.len()),
                SENTINEL_THEME_PICKER => {
                    let value =
                        if state.themes_json_present { state.theme.active_theme.clone() } else { "missing themes.json".to_string() };
                    return suffixed(&app.name, value);
                }
                SENTINEL_EMOJI => {
                    let status = match &state.emoji {
                        Some(data) => format!("Version {}", data.version),
                        None => "missing emoji-test.txt".to_string(),
                    };
                    return suffixed(&app.name, status);
                }
                SENTINEL_COPY_HISTORY => {
                    let status =
                        if state.copy_history_enabled { state.copy_history.len().to_string() } else { "disabled".to_string() };
                    return suffixed(&app.name, status);
                }
                _ => {}
            }
        }
    }
    // "<emoji> <nom>" à l'affichage seulement : `mode_items` (la chaîne
    // filtrée) ne contient QUE le nom. Avec le glyphe en préfixe, "gri" ne
    // matcherait plus "grinning face" en tête (tier 0), la comparaison
    // commençant alors par l'emoji.
    if state.mode == Mode::Emoji {
        if let Some(entry) = state.emoji.as_ref().and_then(|d| d.entries.get(idx)) {
            return format!("{} {}", entry.glyph, entry.name);
        }
    }
    state.mode_items.get(idx).cloned().unwrap_or_default()
}

// --- Filtrage -------------------------------------------------------------

/// `normalized_items` déjà repliés/minuscules -- la normalisation est le
/// coût réel ici (une allocation par élément), faite une fois par
/// changement de liste et non à chaque frappe (voir normalized_mode_items).
fn fuzzy_filter(normalized_items: &[String], query_lower: &str) -> Vec<usize> {
    if query_lower.is_empty() {
        return (0..normalized_items.len()).collect();
    }
    let mut ranked: Vec<(usize, (u8, usize))> = normalized_items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match_rank_multi(s, query_lower).map(|r| (i, r)))
        .collect();
    ranked.sort_by_key(|&(_, rank)| rank);
    ranked.into_iter().map(|(i, _)| i).collect()
}

/// Version normalisée de `state.mode_items`, recalculée seulement si
/// `mode_items` a changé depuis le dernier appel (voir `mode_items_cache`).
fn normalized_mode_items(state: &mut AppState) -> &[String] {
    if state.mode_items_cache.0 != state.mode_items {
        state.mode_items_cache.1 = state.mode_items.iter().map(|s| normalize(s)).collect();
        state.mode_items_cache.0 = state.mode_items.clone();
    }
    &state.mode_items_cache.1
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
    let query_normalized = normalize(trimmed);
    state.filtered = fuzzy_filter(normalized_mode_items(state), &query_normalized);
}

fn current_list_len(state: &AppState) -> usize {
    match state.display {
        SearchDisplay::List => state.filtered.len(),
        _ => 0,
    }
}

/// `true` si la sélection (ou le défilement) a réellement bougé, `false`
/// pour un no-op (liste vide, ou déjà à la butée dans la direction
/// demandée). Le retour pilote l'invalidation côté appelant : sans lui,
/// une flèche maintenue contre une butée redessine la fenêtre en boucle
/// pour un résultat identique.
unsafe fn move_selection(state: &mut AppState, delta: i32) -> bool {
    let len = current_list_len(state);
    if len == 0 {
        return false;
    }
    let old_selected = state.selected;
    let old_first_visible = state.first_visible;
    let new_selected = (state.selected as i32 + delta).clamp(0, len as i32 - 1) as usize;
    state.selected = new_selected;
    if state.selected < state.first_visible {
        state.first_visible = state.selected;
    } else if state.selected >= state.first_visible + VISIBLE_ROWS {
        state.first_visible = state.selected - VISIBLE_ROWS + 1;
    }
    let moved = state.selected != old_selected || state.first_visible != old_first_visible;
    // Preview seulement si la sélection a bougé : sinon une flèche contre
    // une butée re-prévisualise le MÊME thème à chaque frappe
    // (preview_theme + recréation de police/pinceau) pour rien.
    if moved && state.mode == Mode::Theme {
        if let Some(&idx) = state.filtered.get(state.selected) {
            if let Some(name) = state.mode_items.get(idx).cloned() {
                theme::preview_theme(&mut state.theme, &name);
                // Les contrôles EDIT ont leur propre cycle de peinture
                // (voir apply_theme_visuals) : une preview en direct doit
                // les redessiner à CHAQUE flèche, sans quoi la barre de
                // recherche ne suit qu'à la prochaine frappe.
                apply_theme_visuals(state);
            }
        }
    }
    moved
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
        Mode::RecycleBin => rebuild_recyclebin_items(hwnd, state),
        Mode::Emoji => rebuild_emoji_items(state),
        Mode::CopyHistory => rebuild_copy_history_items(state),
        Mode::Eject => rebuild_eject_items(state),
        Mode::Timer => {}
    }
    set_edit_text(state.edit_hwnd, "");
    refresh_placeholder(state);
    // RDW_UPDATENOW plutôt qu'un simple InvalidateRect, même raison que dans
    // apply_theme_visuals : enchaîner vite des changements de mode laisse
    // sinon l'ancien texte/placeholder à l'écran longtemps après que
    // set_edit_text("") ait vidé le contrôle, le WM_PAINT de l'EDIT ne
    // passant jamais devant les WM_KEYDOWN qui continuent d'arriver.
    RedrawWindow(state.edit_hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASE);
    refresh_filter(state);
    if mode == Mode::Theme {
        // refresh_filter remet la sélection à l'index 0 (premier thème dans
        // l'ordre alphabétique) : recalage sur le thème actif, pour que le
        // picker s'ouvre là où on est déjà.
        if let Some(idx) = state.mode_items.iter().position(|name| *name == state.theme.active_theme) {
            state.selected = idx;
            state.first_visible = if idx >= VISIBLE_ROWS { idx - VISIBLE_ROWS + 1 } else { 0 };
        }
    }
    // Un changement de mode change toujours le contenu affiché : invalidé
    // ici une fois pour toutes, plutôt que dans chacun des appelants. C'est
    // ce qui permet à handle_edit_keydown de n'invalider que sur changement
    // réel, sans filet de sécurité aveugle (voir son commentaire).
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

/// Restaure le thème actif d'origine si on quitte le sélecteur sans avoir
/// validé -- partagé par `exit_picker` (Échap) et Tab (le Window Switcher
/// est accessible depuis n'importe quel mode) : quitter Thème sans valider
/// ne doit jamais laisser un thème seulement prévisualisé.
unsafe fn cancel_uncommitted_theme_preview(state: &mut AppState) {
    if state.mode == Mode::Theme {
        if let Some(orig) = state.theme_picker_original.take() {
            theme::preview_theme(&mut state.theme, &orig);
            apply_theme_visuals(state);
        }
    }
}

/// Revient au mode Normal en annulant une preview de thème non validée.
unsafe fn exit_picker(hwnd: HWND, state: &mut AppState) {
    cancel_uncommitted_theme_preview(state);
    enter_mode(hwnd, state, Mode::Normal);
}

// --- Actions ---------------------------------------------------------------

unsafe fn reload_config(hwnd: HWND, state: &mut AppState) {
    if let Ok(cfg) = crate::core::config::load_all(&state.base_dir) {
        state.apps = cfg.apps;
    }
    state.themes_json_present = theme::load(&state.themes_path, &mut state.theme);
    // notes.json/restart.json sont normalement déjà à jour en mémoire (le
    // lanceur est seul à les écrire, voir core::json_list) -- les relire
    // coûte peu et couvre une édition manuelle faite pendant que le lanceur
    // tourne, sinon invisible jusqu'au prochain redémarrage.
    state.notes = crate::core::json_list::load_notes(&state.notes_path);
    state.restart_targets = crate::core::json_list::load_restart_list(&state.restart_path);
    state.restart_supervisor.set_targets(state.restart_targets.clone());
    // Même logique pour emoji-test.txt : permet de déposer une version plus
    // récente d'Unicode sans redémarrer le lanceur.
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
            // Aucune validation de format : toute cible non vide est
            // acceptée telle quelle, arguments compris. Une cible invalide
            // se voit à l'usage plutôt que d'être rejetée a priori.
            if !target.is_empty() {
                state.restart_targets.push(target);
                let _ = crate::core::json_list::save_restart_list(&state.restart_path, &state.restart_targets);
                state.restart_supervisor.set_targets(state.restart_targets.clone());
                rebuild_restart_items(state);
                set_edit_text(state.edit_hwnd, "");
                refresh_filter(state);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
        Mode::Theme => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(name) = state.mode_items.get(idx).cloned() {
                    theme::preview_theme(&mut state.theme, &name);
                    let _ = theme::commit_theme(&state.themes_path, &name);
                    state.theme.active_theme = name;
                    state.theme_picker_original = None;
                    apply_theme_visuals(state);
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
        // Copie le nom complet (extension comprise) de l'élément
        // sélectionné. Vider la Corbeille reste réservé à Maj+Suppr ici, ou
        // à Maj+Entrée/Suppr sur "Empty Recycle Bin" au menu principal.
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
        // Ne ferme pas le lanceur, contrairement aux autres modes : plusieurs
        // périphériques branchés à la fois est courant, autant pouvoir les
        // éjecter à la suite. `force = false` : si un handle est encore
        // ouvert sur le volume (FSCTL_LOCK_VOLUME refusé), Entrée n'insiste
        // pas et l'entrée reste dans la liste -- forcer reste un geste
        // distinct (Maj+Suppr, voir on_delete).
        Mode::Eject => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if eject_selected(state, idx, false) {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
        }
        // Pas d'ajout à la saisie ici, contrairement à Sticky Notes : les
        // entrées viennent uniquement de la capture presse-papier (voir
        // WM_CLIPBOARDUPDATE).
        Mode::CopyHistory => {
            if let Some(&idx) = state.filtered.get(state.selected) {
                if let Some(text) = state.copy_history.get(idx).map(str::to_string) {
                    // Posé AVANT set_clipboard_text : le WM_CLIPBOARDUPDATE
                    // que cette copie va déclencher doit être ignoré, sinon
                    // l'entrée serait réinjectée en doublon en tête.
                    state.suppress_next_clipboard_capture = true;
                    set_clipboard_text(hwnd, &text);
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
                // Rien si themes.json est absent/invalide : l'entrée
                // l'annonce déjà (row_label : "Theme: missing themes.json"),
                // inutile d'ouvrir un sélecteur réduit au thème de secours.
                SENTINEL_THEME_PICKER => {
                    if state.themes_json_present {
                        enter_mode(hwnd, state, Mode::Theme);
                    }
                }
                SENTINEL_TIMER => enter_mode(hwnd, state, Mode::Timer),
                SENTINEL_NOTES => enter_mode(hwnd, state, Mode::Notes),
                SENTINEL_RESTART => enter_mode(hwnd, state, Mode::Restart),
                // Rien si emoji-test.txt est absent : l'entrée l'annonce
                // déjà (row_label : "Emoji: missing emoji-test.txt").
                SENTINEL_EMOJI => {
                    if state.emoji.is_some() {
                        enter_mode(hwnd, state, Mode::Emoji);
                    }
                }
                // Rien si désactivé -- l'entrée l'annonce déjà (voir
                // row_label : "Copy History: disabled"), à activer depuis
                // le menu tray plutôt que depuis ce picker.
                SENTINEL_COPY_HISTORY => {
                    if state.copy_history_enabled {
                        enter_mode(hwnd, state, Mode::CopyHistory);
                    }
                }
                SENTINEL_OPEN_FOLDER => {
                    let _ = crate::core::launch::launch(&state.base_dir.to_string_lossy(), None, false);
                    hide(hwnd);
                }
                // Entrée ouvre la vue de consultation (lecture seule) ;
                // Maj+Entrée (reveal_or_edit) vide la Corbeille -- l'action
                // destructive est réservée au modificateur.
                SENTINEL_EMPTY_RECYCLE_BIN => enter_mode(hwnd, state, Mode::RecycleBin),
                SENTINEL_EJECT => enter_mode(hwnd, state, Mode::Eject),
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
/// (mode Normal), ou ouvre notes.json dans son éditeur associé (mode Notes).
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
                empty_recycle_bin(hwnd, state);
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

/// `true` si Suppr/Maj+Suppr a réellement changé quelque chose, `false`
/// pour un no-op (rien de sélectionné, liste vide, mode où Suppr n'a pas
/// de sens). N'invalide jamais elle-même : l'appelant le fait une fois, une
/// fois la réponse connue -- sinon Suppr maintenue redessine la fenêtre à
/// chaque frappe pour un résultat identique.
unsafe fn on_delete(hwnd: HWND, state: &mut AppState, shift: bool) -> bool {
    match state.mode {
        Mode::Window => {
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            let Some(w) = state.windows.get(idx) else { return false };
            if shift {
                crate::core::windows::kill_window(w.hwnd);
            } else {
                crate::core::windows::close_window(w.hwnd);
            }
            // Suppression optimiste de la liste locale plutôt qu'une
            // ré-énumération : close_window passe par
            // PostMessageW(WM_CLOSE), asynchrone -- un EnumWindows immédiat
            // retrouve presque toujours la fenêtre encore vivante, et Suppr
            // paraîtrait alors sans effet.
            state.windows.remove(idx);
            sync_window_mode_items(state);
            refresh_filter(state);
            true
        }
        Mode::Notes => {
            let changed = if shift {
                let had_notes = !state.notes.is_empty();
                state.notes.clear();
                had_notes
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.notes.len() {
                    state.notes.remove(idx);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !changed {
                return false;
            }
            let _ = crate::core::json_list::save_notes(&state.notes_path, &state.notes);
            rebuild_notes_items(state);
            refresh_filter(state);
            true
        }
        Mode::CopyHistory => {
            let changed = if shift {
                let had_entries = state.copy_history.len() > 0;
                state.copy_history.clear();
                had_entries
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                state.copy_history.remove(idx);
                true
            } else {
                false
            };
            if !changed {
                return false;
            }
            rebuild_copy_history_items(state);
            refresh_filter(state);
            true
        }
        Mode::Restart => {
            let changed = if shift {
                let had_targets = !state.restart_targets.is_empty();
                state.restart_targets.clear();
                had_targets
            } else if let Some(&idx) = state.filtered.get(state.selected) {
                if idx < state.restart_targets.len() {
                    state.restart_targets.remove(idx);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !changed {
                return false;
            }
            let _ = crate::core::json_list::save_restart_list(&state.restart_path, &state.restart_targets);
            state.restart_supervisor.set_targets(state.restart_targets.clone());
            rebuild_restart_items(state);
            refresh_filter(state);
            true
        }
        Mode::RecycleBin => {
            if shift {
                empty_recycle_bin(hwnd, state);
                // Le vidage est asynchrone, mais le résultat est certain :
                // inutile d'attendre un nouveau scan pour que la vue de
                // consultation le reflète.
                state.recycle_bin_items.clear();
                state.mode_items.clear();
                refresh_filter(state);
                return true;
            }
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            let Some(item) = state.recycle_bin_items.get(idx).cloned() else { return false };
            crate::core::recycle_bin::delete_item(&item);
            state.recycle_bin_cache.set(None);
            // Suppression optimiste de la liste locale plutôt qu'un nouveau
            // scan complet (même principe que Mode::Window) : évite de
            // vider la vue le temps qu'un scan en arrière-plan revienne,
            // pour un seul élément déjà supprimé avec certitude.
            state.recycle_bin_items.remove(idx);
            sync_recyclebin_mode_items(state);
            refresh_filter(state);
            true
        }
        Mode::Timer => {
            if state.timer_deadline.is_some() {
                cancel_timer(hwnd, state);
                true
            } else {
                false
            }
        }
        // Suppr seul ne fait rien : Entrée est déjà l'action normale du
        // mode. Maj+Suppr force le démontage même si le volume est encore
        // utilisé (voir disk_ejector::eject_drive), geste volontairement
        // distinct d'Entrée.
        Mode::Eject => {
            if !shift {
                return false;
            }
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            eject_selected(state, idx, true)
        }
        Mode::Normal => {
            let SearchDisplay::List = state.display else { return false };
            let Some(&idx) = state.filtered.get(state.selected) else { return false };
            let Some(app) = state.apps.get(idx) else { return false };
            if app.path == SENTINEL_EMPTY_RECYCLE_BIN {
                empty_recycle_bin(hwnd, state);
                true
            } else if app.path == SENTINEL_TIMER && state.timer_deadline.is_some() {
                cancel_timer(hwnd, state);
                true
            } else if app.path == SENTINEL_NOTES && !state.notes.is_empty() {
                // Retire la note la plus récente (notes[0], même convention
                // que row_label) sans entrer dans le picker Notes -- même
                // esprit que Suppr sur le Timer juste au-dessus.
                state.notes.remove(0);
                let _ = crate::core::json_list::save_notes(&state.notes_path, &state.notes);
                true
            } else if app.path == SENTINEL_COPY_HISTORY && state.copy_history.len() > 0 {
                // Même principe que SENTINEL_NOTES : retire l'entrée la
                // plus récente (index 0) sans entrer dans le picker.
                state.copy_history.remove(0);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

// --- Timer / rebond DVD -----------------------------------------------

unsafe fn arm_timer(hwnd: HWND, state: &mut AppState, seconds: u64) {
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

/// Force la fenêtre au premier plan malgré l'heuristique anti-vol-de-focus
/// de Windows. Un `SetForegroundWindow` appelé hors de tout lien avec une
/// entrée utilisateur récente -- typiquement depuis un WM_TIMER pendant que
/// la fenêtre est masquée, le cas du rebond DVD -- échoue SILENCIEUSEMENT :
/// la fenêtre s'affiche (ShowWindow réussit toujours) mais le focus clavier
/// réel reste sur l'application précédente, et aucune touche, Échap compris,
/// n'atteint la nôtre. L'heuristique dépendant du contexte, l'échec est
/// intermittent. Un hotkey global ou un clic compte comme entrée utilisateur
/// légitime et n'est pas concerné.
/// `AttachThreadInput` fusionne temporairement notre file d'entrée avec
/// celle du thread propriétaire de la fenêtre au premier plan, ce qui
/// satisfait la condition manquante -- technique standard pour ce cas, sans
/// recourir à une simulation de frappe (`SendInput`), évitée dans tout le
/// projet pour ne pas ressembler à un injecteur d'entrée aux yeux d'un
/// antivirus (voir win32::user32).
unsafe fn force_foreground(hwnd: HWND) {
    let foreground = GetForegroundWindow();
    if foreground.is_null() || foreground == hwnd {
        SetForegroundWindow(hwnd);
        return;
    }
    let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
    let current_thread = GetCurrentThreadId();
    if foreground_thread != 0 && foreground_thread != current_thread {
        AttachThreadInput(current_thread, foreground_thread, 1);
        SetForegroundWindow(hwnd);
        AttachThreadInput(current_thread, foreground_thread, 0);
    } else {
        SetForegroundWindow(hwnd);
    }
}

unsafe fn start_bounce(hwnd: HWND, state: &mut AppState) {
    state.bouncing = true;
    state.bounce_pre_geometry = Some(state.geometry);
    state.bounce_pre_theme = Some(state.theme.active_theme.clone());
    state.bounce_pos = (state.geometry.window.left as f64, state.geometry.window.top as f64);
    let angle = state.rng.next_f64() * std::f64::consts::TAU;
    state.bounce_vel = (BOUNCE_SPEED_PX * angle.cos(), BOUNCE_SPEED_PX * angle.sin());
    ShowWindow(hwnd, SW_SHOW);
    force_foreground(hwnd);
    SetActiveWindow(hwnd);
    // Le rebond démarre en général depuis un minuteur qui a couru pendant
    // que la fenêtre était masquée (hide() ne tue pas FIRE_TIMER_ID).
    // Windows ne restaure PAS le focus clavier sur edit_hwnd quand une
    // fenêtre masquée redevient visible/active. Sans ce SetFocus explicite,
    // les WM_KEYDOWN arrivent avec `msg.hwnd == hwnd` au lieu de
    // `== state.edit_hwnd` : la boucle de messages (main.rs) ne les route
    // alors jamais vers handle_edit_keydown, et Échap n'atteint jamais la
    // branche `state.bouncing` qui arrête le rebond.
    SetFocus(state.edit_hwnd);
    SetTimer(hwnd, BOUNCE_TIMER_ID, BOUNCE_INTERVAL_MS, None);
}

/// Repaint synchrone de la fenêtre principale ET de ses contrôles enfants
/// en un appel -- à faire après tout `SetWindowPos` qui change la taille
/// (stop_bounce, show, set_window_size_percent, adjust_border). Un
/// `InvalidateRect` se contente de marquer la zone à repeindre, et son
/// `WM_PAINT` (priorité la plus basse de la file) peut être retardé
/// derrière les messages générés par le `SetWindowPos` lui-même. Avec
/// `WS_EX_COMPOSITED`, la DWM donne à la fenêtre sa propre surface
/// hors-écran, dont les pixels nouvellement exposés par un agrandissement
/// démarrent transparents jusqu'à notre `BitBlt` : si la DWM présente une
/// frame avant ce WM_PAINT différé, la zone agrandie apparaît transparente
/// une frame, d'autant plus visiblement que l'agrandissement est large.
/// `RDW_UPDATENOW` ferme la course en forçant le `WM_PAINT` ici même.
unsafe fn force_repaint_now(hwnd: HWND) {
    RedrawWindow(hwnd, std::ptr::null(), std::ptr::null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASE | RDW_ALLCHILDREN);
}

unsafe fn stop_bounce(hwnd: HWND, state: &mut AppState) {
    state.bouncing = false;
    KillTimer(hwnd, BOUNCE_TIMER_ID);
    if let Some(name) = state.bounce_pre_theme.take() {
        theme::preview_theme(&mut state.theme, &name);
        apply_theme_visuals(state);
    }
    if let Some(geometry) = state.bounce_pre_geometry.take() {
        state.geometry = geometry;
        let g = geometry.window;
        SetWindowPos(hwnd, HWND_TOPMOST, g.left, g.top, rect_w(&g), rect_h(&g), SWP_NOZORDER);
        force_repaint_now(hwnd);
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
            apply_theme_visuals(state);
        }
        // Repaint uniquement quand le contenu change réellement, ici les
        // couleurs. SetWindowPos ne fait que déplacer la fenêtre : la DWM
        // (WS_EX_COMPOSITED, voir create()) recompose l'image déjà rendue à
        // la nouvelle position sans repaint côté appli. Repeindre à chaque
        // tick referait tout le travail de DrawTextW (10 lignes + barre de
        // recherche) 60 fois par seconde -- négligeable avec Segoe UI, mais
        // assez coûteux avec une police aux glyphes complexes pour produire
        // un à-coup visible.
        InvalidateRect(hwnd, std::ptr::null(), 0);
    }
}

// --- Rendu -------------------------------------------------------------

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
unsafe fn draw_scene(hdc: HDC, state: &AppState, dirty: &RECT) {
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

/// Rectangle de la fenêtre entière, en coordonnées clientes -- basé sur les
/// dimensions réelles (`g.window`) et non reconstruit à partir des rects de
/// lignes : les arrondis de la division entière de compute_geometry
/// sous-estimeraient de quelques pixels, laissant un liseré non repeint en
/// bas de fenêtre.
fn full_window_rect(g: &Geometry) -> RECT {
    rect(0, 0, rect_w(&g.window), rect_h(&g.window))
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

// --- Fenêtre / message ---------------------------------------------------

/// (Re)crée `state.mem_dc`/`state.mem_bitmap` pour qu'ils fassent
/// exactement la taille de la fenêtre -- ne fait rien si la taille n'a pas
/// changé depuis le dernier appel (même garde-fou que les polices/pinceaux,
/// voir mem_buffer_size). `hdc_screen` sert uniquement de référence de
/// format de pixels à `CreateCompatibleDC`/`CreateCompatibleBitmap`, jamais
/// dessiné dedans.
unsafe fn ensure_scene_buffer(hdc_screen: HDC, state: &mut AppState) {
    let w = rect_w(&state.geometry.window).max(1);
    let h = rect_h(&state.geometry.window).max(1);
    if state.mem_buffer_size == (w, h) && !state.mem_dc.is_null() {
        return;
    }
    if !state.mem_bitmap.is_null() {
        DeleteObject(state.mem_bitmap as _);
    }
    if !state.mem_dc.is_null() {
        DeleteDC(state.mem_dc);
    }
    state.mem_dc = CreateCompatibleDC(hdc_screen);
    state.mem_bitmap = CreateCompatibleBitmap(hdc_screen, w, h);
    SelectObject(state.mem_dc, state.mem_bitmap as _);
    state.mem_buffer_size = (w, h);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            // Le fond entier est déjà repeint dans draw_scene à chaque
            // WM_PAINT -- laisser l'effacement par défaut ferait clignoter
            // la fenêtre pour rien.
            1
        }
        WM_PAINT => {
            // Dessiné dans state.mem_dc puis présenté d'un seul BitBlt
            // atomique -- voir mem_dc dans AppState.
            if let Some(state) = get_state(hwnd) {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                ensure_scene_buffer(hdc, state);
                draw_scene(state.mem_dc, state, &ps.rcPaint);
                let w = ps.rcPaint.right - ps.rcPaint.left;
                let h = ps.rcPaint.bottom - ps.rcPaint.top;
                if w > 0 && h > 0 {
                    BitBlt(hdc, ps.rcPaint.left, ps.rcPaint.top, w, h, state.mem_dc, ps.rcPaint.left, ps.rcPaint.top, SRCCOPY);
                }
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            // L'EDIT de recherche doit se fondre dans la barre (même fond
            // que search_background). WM_CTLCOLORSTATIC ne concerne aucun
            // contrôle actuel -- l'horloge n'en est pas un, voir
            // draw_clock_text -- mais couvre un futur enfant STATIC.
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
        // Diffusé à tout le système dès qu'un process change le contenu du
        // presse-papier -- reçu ici tant que AddClipboardFormatListener est
        // enregistré sur ce hwnd (voir create()/toggle_copy_history).
        WM_CLIPBOARDUPDATE => {
            if let Some(state) = get_state(hwnd) {
                if state.suppress_next_clipboard_capture {
                    // Message déclenché par notre propre set_clipboard_text
                    // (re-copie d'une entrée de l'historique) : ignoré une
                    // fois, plutôt que de dupliquer l'entrée en tête.
                    state.suppress_next_clipboard_capture = false;
                } else if state.copy_history_enabled && !crate::win32::clipboard_excluded_from_history() {
                    // La source (gestionnaire de mots de passe le plus
                    // souvent) peut demander à ne pas être capturée par un
                    // surveillant de presse-papier. Vérifié à CHAQUE
                    // message, jamais mis en cache : le contenu précédent
                    // pouvait très bien ne pas être exclu.
                    if let Some(text) = get_clipboard_text(hwnd) {
                        state.copy_history.push(text);
                        if state.mode == Mode::CopyHistory {
                            rebuild_copy_history_items(state);
                            refresh_filter(state);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                }
            }
            0
        }
        WM_TIMER => {
            if let Some(state) = get_state(hwnd) {
                match wparam {
                    CLOCK_TIMER_ID => {
                        // Invalidation scopée au seul rect de l'horloge :
                        // ce tick tourne une fois par seconde tant que la
                        // fenêtre est ouverte, inutile de repeindre toute
                        // la scène pour un texte confiné à ce coin.
                        if state.theme.show_clock {
                            InvalidateRect(hwnd, &state.geometry.clock, 0);
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
                    RECYCLE_BIN_POLL_TIMER_ID => poll_recycle_bin(hwnd, state),
                    _ => {}
                }
            }
            0
        }
        // Un clic, quel que soit le bouton, est une des trois sorties du
        // rebond DVD (avec Échap et le raccourci global). La souris n'agit
        // nulle part ailleurs dans la fenêtre : hors rebond, ce bras est
        // un no-op.
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            if let Some(state) = get_state(hwnd) {
                if state.bouncing {
                    stop_bounce(hwnd, state);
                    hide(hwnd);
                }
            }
            0
        }
        // Alt+F4 (et tout SC_CLOSE) atteint la fenêtre par WM_CLOSE. Sans ce
        // bras, DefWindowProcW ferait DestroyWindow sur la fenêtre
        // PRINCIPALE, qui ne doit jamais être détruite avant la sortie du
        // process (seul tray_hwnd l'est, via "Quit"). L'appli resterait
        // alors à moitié vivante : WM_DESTROY libère l'état mais n'appelle
        // pas PostQuitMessage, donc le process survit avec un tray présent
        // et un hotkey pointant sur un HWND détruit. Alt+F4 est donc un
        // simple hide.
        // WM_CLOSE arrive par DispatchMessageW, sans passer par
        // l'interception WM_KEYDOWN de main.rs : il doit arrêter le rebond
        // lui-même, sinon la fenêtre se cache avec `bouncing` à true et
        // BOUNCE_TIMER_ID toujours armé, à repositionner une fenêtre
        // invisible jusqu'au prochain show().
        WM_CLOSE => {
            if let Some(state) = get_state(hwnd) {
                if state.bouncing {
                    stop_bounce(hwnd, state);
                }
            }
            hide(hwnd);
            0
        }
        WM_ACTIVATE => {
            // Perte de focus (bascule vers une autre appli) -> ferme la
            // popup.
            if (wparam & 0xFFFF) as u32 == WA_INACTIVE {
                if let Some(state) = get_state(hwnd) {
                    if state.bouncing {
                        stop_bounce(hwnd, state);
                    } else {
                        // Une preview de thème en cours doit être annulée
                        // ici aussi : sinon elle survit à une perte de
                        // focus (alt-tab, notification) et le prochain
                        // affichage garde les couleurs d'un thème jamais
                        // validé.
                        cancel_uncommitted_theme_preview(state);
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
                KillTimer(hwnd, RECYCLE_BIN_POLL_TIMER_ID);
                state.restart_supervisor.stop();
                if state.copy_history_enabled {
                    RemoveClipboardFormatListener(hwnd);
                }
                if !state.font_row.is_null() {
                    DeleteObject(state.font_row as _);
                }
                if !state.font_search.is_null() {
                    DeleteObject(state.font_search as _);
                }
                if !state.search_brush.is_null() {
                    DeleteObject(state.search_brush as _);
                }
                if !state.list_bg_brush.is_null() {
                    DeleteObject(state.list_bg_brush as _);
                }
                if !state.selected_bg_brush.is_null() {
                    DeleteObject(state.selected_bg_brush as _);
                }
                if !state.border_brush.is_null() {
                    DeleteObject(state.border_brush as _);
                }
                if !state.mem_bitmap.is_null() {
                    DeleteObject(state.mem_bitmap as _);
                }
                if !state.mem_dc.is_null() {
                    DeleteDC(state.mem_dc);
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

/// Procédure d'origine de la classe stock "EDIT", capturée avant le
/// sous-classement (voir `create`). Tout EDIT de cette fenêtre partage la
/// même classe système, donc la même adresse : une capture suffit.
static ORIGINAL_EDIT_WNDPROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

unsafe fn call_original_edit_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let orig = ORIGINAL_EDIT_WNDPROC.load(std::sync::atomic::Ordering::Relaxed);
    let proc: WNDPROC = std::mem::transmute(orig);
    CallWindowProcW(proc, hwnd, msg, wparam, lparam)
}

/// EM_SETMARGINS fixe la marge interne gauche/droite d'un EDIT à une valeur
/// explicite. La marge implicite par défaut varie selon la présence du
/// manifeste comctl32 v6 et n'est pas lisible via EM_GETMARGINS tant
/// qu'elle n'a jamais été posée -- draw_placeholder réutilise donc la
/// valeur posée ici (state.text_margin), et texte tapé et placeholder
/// démarrent au même endroit par construction.
const EM_SETMARGINS: u32 = 0xD3;
const EC_LEFTMARGIN: usize = 0x1;
const EC_RIGHTMARGIN: usize = 0x2;

/// Dessine le texte d'invite à la main quand le champ est vide, avec la
/// police et la couleur du thème courant -- remplace EM_SETCUEBANNER, qui
/// peint avec une couleur interne à comctl32 ignorant SetTextColor et ne
/// suit donc jamais un changement de thème. `hdc` vient de l'appelant
/// plutôt que d'un GetDC local : le placeholder doit rester dans la même
/// session BeginPaint/EndPaint que le fond (voir edit_subclass_proc).
unsafe fn draw_placeholder(hdc: HDC, hwnd: HWND, state: &AppState) {
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

/// Sous-classe du contrôle EDIT de recherche : l'appli est entièrement
/// pilotée au clavier, la souris ne doit rien pouvoir y faire. Filtrer les
/// touches dans `handle_edit_keydown` n'y suffit pas -- un EDIT natif
/// réagit aussi à la souris (caret, sélection, curseur I-beam au survol),
/// indépendamment du clavier. Tout message souris est donc avalé ici avant
/// d'atteindre la procédure d'origine ; le reste (texte, focus, police) lui
/// est transmis normalement.
unsafe extern "system" fn edit_subclass_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK
        | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_MOUSEMOVE | WM_MOUSEWHEEL | WM_NCLBUTTONDOWN => 0,
        WM_SETCURSOR => {
            SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_ARROW));
            1
        }
        // Restreint au champ vide, seul cas où le fond est peint ici (voir
        // WM_PAINT ci-dessous) : dès qu'il y a du texte,
        // call_original_edit_proc garde son propre WM_ERASEBKGND, dont il a
        // besoin pour effacer les anciens glyphes entre deux frappes.
        WM_ERASEBKGND if GetWindowTextLengthW(hwnd) == 0 => 1,
        // Champ vide : fond ET placeholder peints dans la MÊME session
        // BeginPaint/EndPaint plutôt qu'en deux présentations successives,
        // même principe d'atomicité que mem_dc dans AppState. La procédure
        // d'origine n'est donc jamais appelée pour ce WM_PAINT, d'où un
        // effet de bord : c'est normalement elle qui repositionne le caret
        // quand le texte change, responsabilité reprise ici tant que le
        // champ reste vide. SetCaretPos le replace au début du texte (même
        // text_margin que draw_placeholder) ; le Y n'a pas à être
        // recalculé, la boîte est mono-ligne. HideCaret/ShowCaret encadrent
        // le tout : le caret s'affiche en XOR, et peindre par-dessus sans
        // l'avoir caché corromprait le pixel qu'il inverse. Les deux sont
        // des no-op si aucun caret n'est affiché.
        WM_PAINT if GetWindowTextLengthW(hwnd) == 0 => {
            if let Some(state) = get_state(GetParent(hwnd)) {
                HideCaret(hwnd);
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT::default();
                GetClientRect(hwnd, &mut rc);
                FillRect(hdc, &rc, state.search_brush);
                draw_placeholder(hdc, hwnd, state);
                EndPaint(hwnd, &ps);
                let mut caret_pos = POINT::default();
                GetCaretPos(&mut caret_pos);
                SetCaretPos(state.text_margin, caret_pos.y);
                ShowCaret(hwnd);
                return 0;
            }
            call_original_edit_proc(hwnd, msg, wparam, lparam)
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
pub fn create(
    apps: Vec<App>,
    mut theme_cfg: ThemeConfig,
    base_dir: PathBuf,
    auto_restart_enabled: bool,
    copy_history_enabled: bool,
) -> Result<WindowHandles, String> {
    let class_name = to_wstring(WINDOW_CLASS_NAME);
    let window_name = to_wstring("MAGI Launcher");

    unsafe {
        let wc = simple_wndclass(
            class_name.as_ptr(),
            Some(wndproc),
            std::ptr::null_mut(),
            LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
        );
        // ERROR_CLASS_ALREADY_EXISTS toléré, comme dans popup_menu::show().
        // En usage réel create() n'est appelée qu'une fois par process ;
        // sous cargo test en revanche, plusieurs #[test] l'appellent sous le
        // même nom de classe et peuvent tourner en parallèle dans le même
        // process -- une classe déjà enregistrée n'est alors pas une panne.
        const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;
        if RegisterClassExW(&wc) == 0 && crate::win32::last_error() != ERROR_CLASS_ALREADY_EXISTS {
            return Err(format!("RegisterClassExW a échoué (erreur {})", crate::win32::last_error()));
        }

        let themes_path = base_dir.join("themes.json");
        let themes_json_present = theme::load(&themes_path, &mut theme_cfg);

        let work = work_area_under_cursor();
        let geometry = compute_geometry(work, &theme_cfg);
        let g = geometry.window;

        let hwnd = CreateWindowExW(
            // WS_EX_COMPOSITED : demande à la DWM de composer la fenêtre ET
            // son enfant (EDIT de recherche) dans un tampon hors écran
            // commun. Le double buffering manuel de draw_scene ne couvre
            // que ce que la fenêtre dessine elle-même, pas le cycle de
            // peinture indépendant d'un contrôle enfant natif.
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_COMPOSITED,
            class_name.as_ptr(),
            window_name.as_ptr(),
            // WS_CLIPCHILDREN : sans ce style, le rendu GDI de la fenêtre
            // (fond de bordure sur tout le client, voir draw_scene) peint
            // PAR-DESSUS le contrôle enfant au lieu d'être découpé autour.
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

        let edit_rect = search_control_rect(&geometry);

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

        // Sous-classement souris (voir edit_subclass_proc).
        ORIGINAL_EDIT_WNDPROC
            .store(GetWindowLongPtrW(edit_hwnd, GWLP_WNDPROC), std::sync::atomic::Ordering::Relaxed);
        SetWindowLongPtrW(edit_hwnd, GWLP_WNDPROC, edit_subclass_proc as *const () as isize);

        let notes_path = base_dir.join("notes.json");
        let restart_path = base_dir.join("restart.json");
        let notes = crate::core::json_list::load_notes(&notes_path);
        let restart_targets = crate::core::json_list::load_restart_list(&restart_path);
        let mut restart_supervisor = RestartSupervisor::new(restart_targets.clone());
        if auto_restart_enabled {
            restart_supervisor.start();
        }
        let emoji = crate::core::emoji::load(&base_dir.join("emoji-test.txt"));

        let mut state = Box::new(AppState {
            mode_items: apps.iter().map(|a| a.name.clone()).collect(),
            mode_items_cache: (Vec::new(), Vec::new()),
            filtered: (0..apps.len()).collect(),
            apps,
            windows: Vec::new(),
            recycle_bin_items: Vec::new(),
            recycle_bin_pending: None,
            eject_drives: Vec::new(),
            emoji,
            notes,
            notes_path,
            restart_targets,
            restart_path,
            restart_supervisor,
            auto_restart_enabled,
            copy_history: crate::core::clipboard_history::ClipboardHistory::new(),
            copy_history_enabled,
            suppress_next_clipboard_capture: false,
            mode: Mode::Normal,
            selected: 0,
            first_visible: 0,
            display: SearchDisplay::List,
            theme: theme_cfg,
            theme_picker_original: None,
            themes_json_present,
            base_dir,
            themes_path,
            timer_deadline: None,
            bouncing: false,
            bounce_pos: (0.0, 0.0),
            bounce_vel: (0.0, 0.0),
            bounce_pre_geometry: None,
            bounce_pre_theme: None,
            rng: SimpleRng::new(),
            edit_hwnd,
            geometry,
            font_row: std::ptr::null_mut(),
            font_search: std::ptr::null_mut(),
            applied_font_family: String::new(),
            applied_font_px: 0,
            search_brush: std::ptr::null_mut(),
            list_bg_brush: std::ptr::null_mut(),
            selected_bg_brush: std::ptr::null_mut(),
            border_brush: std::ptr::null_mut(),
            mem_dc: std::ptr::null_mut(),
            mem_bitmap: std::ptr::null_mut(),
            mem_buffer_size: (0, 0),
            placeholder_wide: Vec::new(),
            text_margin: 0,
            recycle_bin_cache: std::cell::Cell::new(None),
        });
        apply_theme_visuals(&mut state);
        // Armé même si `show_clock` est faux au démarrage : un Reload peut
        // l'activer, et le tick reste un no-op d'ici là (voir
        // CLOCK_TIMER_ID).
        SetTimer(hwnd, CLOCK_TIMER_ID, 1000, None);

        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        if copy_history_enabled {
            AddClipboardFormatListener(hwnd);
        }

        Ok(WindowHandles { main: hwnd, edit: edit_hwnd })
    }
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
unsafe fn apply_geometry(hwnd: HWND, state: &mut AppState) {
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
/// aussitôt dans themes.json, même commit immédiat que le sélecteur de
/// thème. L'écriture disque est best-effort (themes.json en lecture seule,
/// par exemple) : l'affichage a déjà changé, un échec de persistance n'a
/// pas à faire échouer l'action visible.
unsafe fn set_window_size_percent(hwnd: HWND, state: &mut AppState, percent: i32) {
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
    let _ = theme::commit_window_size(&state.themes_path, percent);
}

/// Ctrl+-/Ctrl+= (voir handle_edit_keydown) : ajuste l'épaisseur de bordure
/// de `delta` px et la persiste tout de suite, même principe que
/// set_window_size_percent ci-dessus -- y compris le même garde-fou contre
/// un no-op (bordure déjà à 0 ou déjà à sa borne haute).
unsafe fn adjust_border(hwnd: HWND, state: &mut AppState, delta: i32) {
    let new_border = (state.theme.border_width + delta).clamp(0, 100);
    if new_border == state.theme.border_width {
        return;
    }
    state.theme.border_width = new_border;
    apply_geometry(hwnd, state);
    force_repaint_now(hwnd);
    let _ = theme::commit_border(&state.themes_path, state.theme.border_width);
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
    force_repaint_now(hwnd);
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
        // Le raccourci global est une des trois sorties du rebond DVD (avec
        // Échap et le clic). Sans ce test, hide() masquerait la fenêtre en
        // laissant `bouncing` à true et BOUNCE_TIMER_ID armé, à
        // repositionner une fenêtre invisible jusqu'au prochain show().
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
    get_state(hwnd).is_none_or(|state| state.auto_restart_enabled)
}

/// Bascule le superviseur Auto-restart (menu du tray) : stop()/start()
/// plutôt que de vider `restart_targets`, pour conserver la liste surveillée
/// pendant la désactivation.
pub unsafe fn toggle_auto_restart(hwnd: HWND) {
    if let Some(state) = get_state(hwnd) {
        if state.auto_restart_enabled {
            state.restart_supervisor.stop();
        } else {
            state.restart_supervisor.start();
        }
        state.auto_restart_enabled = !state.auto_restart_enabled;
        // Persisté aussitôt (même principe que window_size/border) pour que
        // la bascule survive à un redémarrage. Best-effort.
        let _ = crate::core::config::commit_bool_setting(
            &state.base_dir.join("apps.json"),
            "auto_restart_enabled",
            state.auto_restart_enabled,
        );
    }
}

pub unsafe fn is_copy_history_enabled(hwnd: HWND) -> bool {
    get_state(hwnd).is_some_and(|state| state.copy_history_enabled)
}

/// Bascule l'historique de presse-papier (menu du tray) -- même persistance
/// immédiate que toggle_auto_restart. Enregistre/retire le listener au même
/// moment : désactiver coupe la capture (plus aucun WM_CLIPBOARDUPDATE
/// reçu) mais ne vide jamais l'historique déjà accumulé -- seule une
/// Suppr/Maj+Suppr dans Mode::CopyHistory le fait.
pub unsafe fn toggle_copy_history(hwnd: HWND) {
    if let Some(state) = get_state(hwnd) {
        state.copy_history_enabled = !state.copy_history_enabled;
        if state.copy_history_enabled {
            AddClipboardFormatListener(hwnd);
        } else {
            RemoveClipboardFormatListener(hwnd);
        }
        let _ = crate::core::config::commit_bool_setting(
            &state.base_dir.join("apps.json"),
            "copy_history_enabled",
            state.copy_history_enabled,
        );
    }
}

/// Appelé par la boucle de messages AVANT TranslateMessage/DispatchMessage
/// pour tout WM_KEYDOWN destiné au contrôle EDIT -- `true` si la touche a
/// été traitée ici (ne doit PAS atteindre l'EDIT), `false` pour la laisser
/// suivre son chemin normal (saisie de texte, Ctrl+C...). Évite d'avoir à
/// sous-classer le contrôle EDIT pour un besoin aussi ciblé.
pub unsafe fn handle_edit_keydown(hwnd: HWND, vk: u16) -> bool {
    let Some(state) = get_state(hwnd) else { return false };

    // Pendant le rebond DVD, Échap suit la règle générale (voir plus bas) :
    // arrête le rebond et laisse la popup visible en mode Normal. Toute
    // autre touche la referme -- le rebond n'a aucun autre usage de la
    // saisie clavier.
    if state.bouncing {
        stop_bounce(hwnd, state);
        if vk == VK_ESCAPE {
            enter_mode(hwnd, state, Mode::Normal);
            SetFocus(state.edit_hwnd);
            InvalidateRect(hwnd, std::ptr::null(), 0);
        } else {
            hide(hwnd);
        }
        return true;
    }

    let ctrl_down = (GetKeyState(VK_CONTROL as i32) as u16) & 0x8000 != 0;
    let shift_down = (GetKeyState(VK_SHIFT as i32) as u16) & 0x8000 != 0;

    // Ctrl+1..9/0 -> window_size 10%..100%, Ctrl+-/Ctrl+= -> bordure ∓1px.
    // Traités AVANT le remap Ctrl+S/W/D/A ci-dessous : ces touches n'ont
    // aucune signification hors Ctrl (un "1"/"-"/"=" seul reste une frappe
    // de recherche), ce ne sont donc pas des alias de touches existantes.
    if ctrl_down {
        let size_percent = match vk {
            VK_1 => Some(10),
            VK_2 => Some(20),
            VK_3 => Some(30),
            VK_4 => Some(40),
            VK_5 => Some(50),
            VK_6 => Some(60),
            VK_7 => Some(70),
            VK_8 => Some(80),
            VK_9 => Some(90),
            VK_0 => Some(100),
            _ => None,
        };
        if let Some(percent) = size_percent {
            set_window_size_percent(hwnd, state, percent);
            return true;
        }
        if vk == VK_OEM_MINUS {
            adjust_border(hwnd, state, -1);
            return true;
        }
        if vk == VK_OEM_PLUS {
            adjust_border(hwnd, state, 1);
            return true;
        }
    }

    // Ctrl+S/W/D/A : alias des flèches (navigation sans quitter le pavé de
    // lettres), normalisés ici plutôt que dupliqués dans le match ci-dessous.
    let vk = match (ctrl_down, vk) {
        (true, VK_S) => VK_DOWN,
        (true, VK_W) => VK_UP,
        (true, VK_D) => VK_RIGHT,
        (true, VK_A) => VK_LEFT,
        _ => vk,
    };

    // Chaque action ci-dessous est responsable de son propre repaint, et
    // seulement quand elle a réellement changé quelque chose
    // (move_selection/on_delete renvoient un bool en ce sens, enter_mode
    // s'invalide lui-même, hide() n'a rien à invalider). Pas de filet de
    // sécurité "on invalide au cas où" ici : il ferait redessiner toute la
    // fenêtre à chaque frappe d'une touche sans effet (Suppr sur liste
    // vide, Ctrl+1 déjà à 10%), source de flashs sous répétition rapide.
    let old_selected = state.selected;
    let old_first_visible = state.first_visible;

    match vk {
        // Haut/Bas d'une ligne, Gauche/Droite d'une page.
        VK_DOWN | VK_UP | VK_LEFT | VK_RIGHT => {
            let delta = match vk {
                VK_DOWN => 1,
                VK_UP => -1,
                VK_RIGHT => VISIBLE_ROWS as i32,
                _ => -(VISIBLE_ROWS as i32),
            };
            if move_selection(state, delta) {
                invalidate_after_navigation(hwnd, state, old_selected, old_first_visible);
            }
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
        // Règle unique quel que soit le mode : Échap ramène au menu
        // principal, et ferme la popup si on y est déjà. `exit_picker`
        // couvre tous les modes et annule une preview de thème non validée.
        VK_ESCAPE => {
            if state.mode == Mode::Normal {
                hide(hwnd);
            } else {
                exit_picker(hwnd, state);
            }
            true
        }
        // Règle unique quel que soit le mode : Tab va au Window Switcher.
        VK_TAB => {
            cancel_uncommitted_theme_preview(state);
            enter_mode(hwnd, state, Mode::Window);
            true
        }
        VK_DELETE => {
            if on_delete(hwnd, state, shift_down) {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            true
        }
        _ => false,
    }
}

/// N'invalide que les rects réellement affectés par un déplacement de
/// sélection (le cas de loin le plus fréquent, flèche maintenue), avec deux
/// retombées vers une invalidation plus large quand plus de deux lignes
/// changent : le défilement et le mode Thème.
unsafe fn invalidate_after_navigation(hwnd: HWND, state: &AppState, old_selected: usize, old_first_visible: usize) {
    if state.mode == Mode::Theme {
        // Chaque flèche prévisualise un thème différent, qui change aussi
        // search_background/border et pas seulement les couleurs des
        // lignes : rien de plus fin qu'un repaint plein-fenêtre ici.
        InvalidateRect(hwnd, std::ptr::null(), 0);
        return;
    }
    if state.first_visible != old_first_visible {
        // Défilement : toutes les lignes visibles changent de contenu
        // (chaque case affiche `first_visible + slot`), leur repaint est
        // donc nécessaire. Bordure, barre de recherche, horloge et
        // séparateur ne bougent jamais en scrollant : les exclure rend un
        // défilement continu nettement moins coûteux par frame.
        let band = rows_band_rect(&state.geometry);
        InvalidateRect(hwnd, &band, 0);
        return;
    }
    if old_selected >= old_first_visible && old_selected - old_first_visible < VISIBLE_ROWS {
        let r = state.geometry.rows[old_selected - old_first_visible];
        InvalidateRect(hwnd, &r, 0);
    }
    if state.selected >= state.first_visible && state.selected - state.first_visible < VISIBLE_ROWS {
        let r = state.geometry.rows[state.selected - state.first_visible];
        InvalidateRect(hwnd, &r, 0);
    }
}

/// Rect englobant les VISIBLE_ROWS lignes -- toutes partagent le même
/// left/width (voir compute_geometry), seul leur Y diffère : du haut de la
/// première au bas de la dernière les couvre exactement, sans la bordure ni
/// la barre de recherche au-dessus.
fn rows_band_rect(g: &Geometry) -> RECT {
    let first = g.rows[0];
    let last = g.rows[VISIBLE_ROWS - 1];
    rect(first.left, first.top, rect_w(&first), last.bottom - first.top)
}

/// Retour arrière sur une recherche déjà vide : sort du picker actif plutôt
/// que de ne rien faire. Point d'entrée distinct de handle_edit_keydown car
/// appelé avant Translate/Dispatch, quand le texte de l'EDIT n'a pas encore
/// changé.
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

/// Stress test et profilage mémoire de la fenêtre, sur un catalogue et un
/// thème synthétiques dans un dossier temporaire (jamais les vrais
/// apps.json/notes.json/restart.json). La fenêtre est pilotée par appel
/// direct de ses fonctions internes -- jamais SendInput/PostMessage, aucune
/// simulation d'événement OS -- et n'est jamais affichée (pas de show()).
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
    use crate::win32::gdi32::{GetDC, ReleaseDC};
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
        // Chemin au-delà de MAX_PATH : exerce la troncature du rendu
        // (DrawTextW + DT_END_ELLIPSIS).
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

        // Copie le vrai themes.json (100+ thèmes) pour exercer le sélecteur
        // sur un catalogue réel plutôt que sur l'unique thème de repli.
        let real_themes = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("themes.json");
        if real_themes.exists() {
            let _ = std::fs::copy(&real_themes, sandbox.join("themes.json"));
        }

        let apps = fake_apps(8);
        let handles =
            create(apps, ThemeConfig::default(), sandbox.clone(), true, false).expect("création fenêtre échouée");
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
                match i % 7 {
                    0 => {
                        // Thème : parcourt tout le catalogue réel avec
                        // preview live à chaque déplacement.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::Theme);
                        let len = current_list_len(get_state(hwnd).unwrap());
                        for _ in 0..len.min(50) {
                            move_selection(get_state(hwnd).unwrap(), 1);
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
                        move_selection(get_state(hwnd).unwrap(), 3);
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
                    5 => {
                        // Corbeille : lecture seule -- Entrée/Suppr y sont
                        // des no-ops voulus, donc sans risque. Le scan
                        // tourne sur un thread dédié : on laisse le temps au
                        // timer de sondage de boucler au moins une fois,
                        // plutôt que de ne tester que la liste vide.
                        let state = get_state(hwnd).unwrap();
                        enter_mode(hwnd, state, Mode::RecycleBin);
                        pump_messages(hwnd, 150);
                        move_selection(get_state(hwnd).unwrap(), 2);
                        launch_selected(hwnd, get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                    _ => {
                        // Copy History : ajout puis suppression réels (RAM
                        // seulement), même esprit que Notes. push() est
                        // appelé directement, jamais via le presse-papier
                        // système : aucun événement OS n'est simulé ici.
                        let state = get_state(hwnd).unwrap();
                        let entry_text = format!("stress-copy-{i}");
                        state.copy_history.push(entry_text.clone());
                        enter_mode(hwnd, state, Mode::CopyHistory);
                        assert!(
                            get_state(hwnd).unwrap().mode_items.iter().any(|s| s == &entry_text),
                            "entrée copy-history '{entry_text}' absente de mode_items"
                        );
                        set_edit_text(get_state(hwnd).unwrap().edit_hwnd, &entry_text);
                        refresh_filter(get_state(hwnd).unwrap());
                        on_delete(hwnd, get_state(hwnd).unwrap(), false);
                        assert!(
                            !get_state(hwnd).unwrap().mode_items.iter().any(|s| s == &entry_text),
                            "entrée copy-history '{entry_text}' jamais supprimée"
                        );
                        exit_picker(hwnd, get_state(hwnd).unwrap());
                    }
                }
            }

            // Redimensionnement en direct (Ctrl+1..9/0 et Ctrl+-/+ en usage
            // réel), appelé directement et non via handle_edit_keydown :
            // GetKeyState y lit l'état du clavier physique, non simulable
            // sans SendInput (proscrit ici). Peu fréquent -- chaque appel
            // écrit sur disque -- et cyclé sur quelques tailles.
            if i % 40 == 0 {
                unsafe {
                    let state = get_state(hwnd).unwrap();
                    set_window_size_percent(hwnd, state, [10, 50, 90][i / 40 % 3]);
                    adjust_border(hwnd, get_state(hwnd).unwrap(), if i % 80 == 0 { 1 } else { -1 });
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
            // exerce la branche WM_ACTIVATE/WA_INACTIVE.
            unsafe {
                if i % 15 == 0 {
                    let state = get_state(hwnd).unwrap();
                    enter_mode(hwnd, state, Mode::Theme);
                    move_selection(get_state(hwnd).unwrap(), 1);
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

        // Une croissance linéaire du nombre d'objets GDI/USER avec les
        // itérations trahirait une fuite de handle (police/pinceau jamais
        // détruits) -- marge x3 pour tolérer la variance normale sans être
        // aveugle à une vraie fuite.
        assert!(
            (final_gdi as u64) <= (baseline_gdi.max(20) as u64) * 3,
            "fuite d'objets GDI suspectée : {baseline_gdi} -> {final_gdi}"
        );
    }

    /// Régression sur l'invariant : aucun handle GDI n'est NULL après une
    /// transition de taille 30% -> 100% -> 30%, ni après les actualisations
    /// qui suivent (défilement, tick de minuteur, Suppr, Tab). Le
    /// clignotement historiquement observé sur ces transitions venait d'une
    /// autre cause -- draw_scene peignant en plusieurs étapes non atomiques
    /// sur le DC de la fenêtre, voir `mem_dc` dans AppState -- mais
    /// l'invariant vérifié ici reste utile à protéger.
    #[test]
    fn transition_30_100_30_ne_laisse_jamais_un_handle_gdi_nul() {
        let sandbox = sandbox_dir("gdi_30_100_30");
        let apps = fake_apps(8);
        let handles =
            create(apps, ThemeConfig::default(), sandbox.clone(), true, false).expect("création fenêtre échouée");
        let hwnd = handles.main;

        // ensure_scene_buffer appelée directement (ce que fait WM_PAINT)
        // plutôt qu'en attendant un vrai WM_PAINT : cette fenêtre de test
        // n'est jamais montrée, et Windows ne dispatche pas WM_PAINT de
        // façon fiable pour une fenêtre jamais affichée, même avec
        // RDW_UPDATENOW. GetDC(NULL) ne sert que de référence de format de
        // pixels à CreateCompatibleDC/CreateCompatibleBitmap.
        unsafe fn assert_handles_valides(hwnd: HWND, label: &str) {
            unsafe {
                let state = get_state(hwnd).expect("state manquant");
                let screen_dc = GetDC(std::ptr::null_mut());
                ensure_scene_buffer(screen_dc, state);
                ReleaseDC(std::ptr::null_mut(), screen_dc);
            }
            unsafe {
                let state = get_state(hwnd).expect("state manquant");
                assert!(!state.font_row.is_null(), "{label}: font_row NULL");
                assert!(!state.font_search.is_null(), "{label}: font_search NULL");
                assert!(!state.search_brush.is_null(), "{label}: search_brush NULL");
                assert!(!state.list_bg_brush.is_null(), "{label}: list_bg_brush NULL");
                assert!(!state.selected_bg_brush.is_null(), "{label}: selected_bg_brush NULL");
                assert!(!state.border_brush.is_null(), "{label}: border_brush NULL");
                assert!(!state.mem_dc.is_null(), "{label}: mem_dc NULL");
                assert!(!state.mem_bitmap.is_null(), "{label}: mem_bitmap NULL");
                let expected = (rect_w(&state.geometry.window).max(1), rect_h(&state.geometry.window).max(1));
                assert_eq!(state.mem_buffer_size, expected, "{label}: mem_buffer_size ne suit pas la géométrie");
            }
            eprintln!("{label}: gdi={} user={} mem={}KB", gdi_object_count(), user_object_count(), process_memory_kb());
        }

        unsafe {
            let state = get_state(hwnd).unwrap();
            set_window_size_percent(hwnd, state, 30);
            assert_handles_valides(hwnd, "apres 30%");

            let state = get_state(hwnd).unwrap();
            set_window_size_percent(hwnd, state, 100);
            assert_handles_valides(hwnd, "apres 100%");

            let state = get_state(hwnd).unwrap();
            set_window_size_percent(hwnd, state, 30);
            assert_handles_valides(hwnd, "apres retour a 30%");

            // Les actualisations à couvrir dans cet état : défilement
            // (flèches), tick de minuteur (InvalidateRect direct, comme
            // WM_TIMER(CLOCK_TIMER_ID)), Suppr, Tab.
            for _ in 0..20 {
                handle_edit_keydown(hwnd, VK_DOWN);
            }
            InvalidateRect(hwnd, std::ptr::null(), 0); // tick d'horloge
            handle_edit_keydown(hwnd, VK_DELETE);
            handle_edit_keydown(hwnd, VK_TAB);
            handle_edit_keydown(hwnd, VK_ESCAPE);
            assert_handles_valides(hwnd, "apres scroll/timer/sup/tab");

            crate::win32::user32::DestroyWindow(hwnd);
        }
        let _ = std::fs::remove_dir_all(&sandbox);
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
