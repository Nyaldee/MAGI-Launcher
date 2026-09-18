//! Minuteur (Mode::Timer) et rebond DVD qui lui succède : identifiants de
//! timer Win32, générateur pseudo-aléatoire local, et toute la mécanique de
//! premier-plan/focus partagée par l'affichage normal et le rebond.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::win32::gdi32::InvalidateRect;
use crate::win32::kernel32::{AttachThreadInput, GetCurrentThreadId};
use crate::win32::user32::{
    GetForegroundWindow, GetWindowThreadProcessId, KillTimer, SetActiveWindow, SetFocus, SetForegroundWindow,
    SetTimer, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOWNA,
};
use crate::win32::HWND;

use crate::ui::gdi::{rect_h, rect_w};
use crate::ui::theme;

use super::app_state::{apply_theme_visuals, AppState, Mode};
use super::force_repaint_now;
use super::geometry::work_area_under_cursor;
use super::mode::enter_mode;

pub(crate) const CLOCK_TIMER_ID: usize = 1;
/// Battement d'une seconde du décompte : anime l'affichage ET déclenche le
/// tir quand `timer_deadline` est atteint (voir `fire`, appelé depuis son
/// bras dans wndproc). Pas de timer distinct pour le tir -- `timer_deadline`
/// (horloge monotone) est l'unique source de vérité, le tir n'est qu'une
/// comparaison lue à chaque battement, donc aucune course KillTimer / message
/// déjà en file.
pub(crate) const COUNTDOWN_TIMER_ID: usize = 2;
pub(crate) const BOUNCE_TIMER_ID: usize = 4;
pub(crate) const RECYCLE_BIN_POLL_TIMER_ID: usize = 5;
pub(crate) const BOUNCE_INTERVAL_MS: u32 = 16;
/// px/tick à ~60fps (16ms). Valeur fixe en pixels, pas une fraction de la
/// largeur d'écran : la vitesse perçue doit rester la même quelle que soit
/// la taille du moniteur.
const BOUNCE_SPEED_PX: f64 = 26.0;

/// Générateur pseudo-aléatoire (xorshift64*) -- direction initiale du
/// rebond DVD et choix du thème à chaque collision. Aucun besoin
/// cryptographique, donc pas de dépendance externe.
pub(crate) struct SimpleRng(u64);

impl SimpleRng {
    pub(crate) fn new() -> Self {
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

pub(crate) unsafe fn arm_timer(hwnd: HWND, state: &mut AppState, seconds: u64) {
    state.timer_deadline = Some(Instant::now() + Duration::from_secs(seconds));
    KillTimer(hwnd, COUNTDOWN_TIMER_ID);
    SetTimer(hwnd, COUNTDOWN_TIMER_ID, 1000, None);
}

pub(crate) unsafe fn cancel_timer(hwnd: HWND, state: &mut AppState) {
    state.timer_deadline = None;
    KillTimer(hwnd, COUNTDOWN_TIMER_ID);
    InvalidateRect(hwnd, std::ptr::null(), 0);
}

/// Le décompte a atteint zéro. Appelé depuis le bras `COUNTDOWN_TIMER_ID` de
/// wndproc quand `Instant::now() >= timer_deadline` -- ce n'est plus un
/// gestionnaire de timer à part entière : arrête le décompte, remet
/// `timer_deadline` à None (invariant : décompte actif ⇔ `is_some()`, un
/// battement en doublon trouve alors None et n'a rien à faire), puis enchaîne
/// sur le rebond DVD.
pub(crate) unsafe fn fire(hwnd: HWND, state: &mut AppState) {
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

/// Affiche la fenêtre au premier plan avec le focus clavier sur l'EDIT --
/// séquence partagée par `show()` et `start_bounce()`. Les deux passaient
/// auparavant par des chemins légèrement différents (`show()` via un
/// `SetForegroundWindow` direct, `start_bounce()` via `force_foreground`) ;
/// unifiés ici sur `force_foreground`, la version robuste aux appels hors
/// contexte d'entrée utilisateur récent (voir son commentaire) -- le cas de
/// `start_bounce()` (déclenché depuis WM_TIMER), mais tout aussi valide
/// pour `show()`.
pub(crate) unsafe fn bring_to_foreground(hwnd: HWND, state: &AppState) {
    // SW_SHOWNA (visible sans activer) plutôt que SW_SHOW : ce dernier
    // tente aussi sa propre activation, en concurrence avec celle de
    // `force_foreground` juste après -- si Windows la bloque (heuristique
    // anti-vol-de-focus, typiquement juste après un Alt-Tab), la fenêtre
    // reste visible mais inactive. SW_SHOWNA laisse `force_foreground` seul
    // responsable de l'activation.
    ShowWindow(hwnd, SW_SHOWNA);
    force_foreground(hwnd);
    SetActiveWindow(hwnd);
    SetFocus(state.edit_hwnd);
}

pub(crate) unsafe fn start_bounce(hwnd: HWND, state: &mut AppState) {
    state.bouncing = true;
    state.bounce_pre_geometry = Some(state.geometry);
    state.bounce_pre_theme = Some(state.theme.active_theme.clone());
    state.bounce_pos = (state.geometry.window.left as f64, state.geometry.window.top as f64);
    let angle = state.rng.next_f64() * std::f64::consts::TAU;
    state.bounce_vel = (BOUNCE_SPEED_PX * angle.cos(), BOUNCE_SPEED_PX * angle.sin());
    // Le rebond démarre en général depuis un décompte qui a couru pendant
    // que la fenêtre était masquée (hide() ne tue pas COUNTDOWN_TIMER_ID).
    // Windows ne restaure PAS le focus clavier sur edit_hwnd quand une
    // fenêtre masquée redevient visible/active. `bring_to_foreground` pose
    // ce focus explicitement : sans lui, les WM_KEYDOWN arrivent avec
    // `msg.hwnd == hwnd` au lieu de `== state.edit_hwnd` -- la boucle de
    // messages (main.rs) ne les route alors jamais vers
    // handle_edit_keydown, et Échap n'atteint jamais la branche
    // `state.bouncing` qui arrête le rebond.
    bring_to_foreground(hwnd, state);
    // `bring_to_foreground` peut délivrer un WM_ACTIVATE(WA_INACTIVE)
    // réentrant (SetForegroundWindow échoue hors contexte d'entrée récente,
    // voir force_foreground -- Windows réaffirme alors l'ancien premier plan
    // en plein milieu de cet appel). Le gestionnaire WM_ACTIVATE voit
    // `bouncing` déjà à true et appelle stop_bounce() ICI, avant qu'on
    // revienne dans cette fonction -- armer le timer sans revérifier
    // `bouncing` le réarmerait alors que stop_bounce vient de le tuer,
    // désynchronisant `bouncing` (false) et BOUNCE_TIMER_ID (armé) : la
    // fenêtre masquée se repositionne dans le vide, et réapparaît en plein
    // rebond dès le prochain show() sans que rien ne sache l'arrêter.
    if state.bouncing {
        SetTimer(hwnd, BOUNCE_TIMER_ID, BOUNCE_INTERVAL_MS, None);
    }
}

pub(crate) unsafe fn stop_bounce(hwnd: HWND, state: &mut AppState) {
    state.bouncing = false;
    KillTimer(hwnd, BOUNCE_TIMER_ID);
    if let Some(name) = state.bounce_pre_theme.take() {
        theme::preview_theme(&mut state.theme, &name);
        apply_theme_visuals(state);
    }
    if let Some(geometry) = state.bounce_pre_geometry.take() {
        state.geometry = geometry;
        let g = geometry.window;
        // Repositionnement pur : la fenêtre est WS_EX_TOPMOST par style et
        // SWP_NOZORDER conserve sa place dans la bande topmost, donc le
        // paramètre hwndInsertAfter est ignoré (null). SWP_NOACTIVATE : ne
        // pas voler l'activation au passage -- l'appelant (show/Échap/hide)
        // gère le focus juste après.
        SetWindowPos(hwnd, std::ptr::null_mut(), g.left, g.top, rect_w(&g), rect_h(&g), SWP_NOZORDER | SWP_NOACTIVATE);
        force_repaint_now(hwnd);
    }
}

pub(crate) unsafe fn bounce_tick(hwnd: HWND, state: &mut AppState) {
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
    // Ce tick ne fait que DÉPLACER : hwndInsertAfter ignoré (SWP_NOZORDER,
    // topmost tenu par le style), et SWP_NOACTIVATE pour ne pas ré-activer la
    // fenêtre 60 fois/s -- sans ça chaque frame reprend l'activation à ce que
    // l'utilisateur vient d'ouvrir (menu tray, autre fenêtre).
    SetWindowPos(hwnd, std::ptr::null_mut(), x.round() as i32, y.round() as i32, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
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
