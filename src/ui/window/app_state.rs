//! `AppState` (l'état complet de la fenêtre, porté par GWLP_USERDATA), les
//! énumérations `Mode`/`SearchDisplay`, et les quelques accesseurs qui
//! opèrent directement dessus sans appartenir à un groupe plus spécifique
//! (thème visuel, texte de l'EDIT, cache Corbeille).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::core::clipboard_history::ClipboardHistory;
use crate::core::disk_ejector::EjectableDrive;
use crate::core::emoji::EmojiData;
use crate::core::models::App;
use crate::core::recycle_bin::RecycleBinItem;
use crate::core::supervisor::ProcessSupervisor;
use crate::core::windows::WindowEntry;
use crate::win32::gdi32::{CreateSolidBrush, DeleteObject, RedrawWindow, HBITMAP, HDC, HFONT, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW};
use crate::win32::user32::{GetWindowTextLengthW, GetWindowTextW, SendMessageW, SetWindowTextW, HBRUSH, WM_SETFONT};
use crate::win32::{from_wstring, to_wstring, HWND};

use crate::ui::gdi::{make_font, rect_h};
use crate::ui::theme::{self, ThemeConfig};

use super::geometry::{text_margin_px, Geometry};
use super::timer::SimpleRng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    Normal,
    Window,
    Timer,
    Notes,
    Restart,
    Kill,
    Theme,
    RecycleBin,
    Emoji,
    CopyHistory,
    Eject,
}

/// Taille de police plancher (voir apply_theme_visuals) -- en-dessous, un
/// EDIT natif ne centre plus fiablement son texte.
const MIN_FONT_PX: i32 = 9;

/// Remplacent `theme.placeholder_text` dans les modes qui attendent une
/// saisie plutôt qu'une recherche. Les modes réellement de recherche
/// (Window Switcher, Thème, Corbeille, Emoji) gardent le placeholder du
/// thème.
const TIMER_PLACEHOLDER: &str = "Type a duration (5m, 90s, 1h...)";
const NOTES_PLACEHOLDER: &str = "Type a note...";
const RESTART_PLACEHOLDER: &str = "Type a target to watch...";
const KILL_PLACEHOLDER: &str = "Type a target to kill...";

pub(crate) enum SearchDisplay {
    List,
    Calc(String),
    Color(u32),
    /// Une seule ligne mise en avant -- aperçu/compte à rebours du Timer.
    SingleLine(String),
}

pub(crate) struct AppState {
    pub(crate) apps: Vec<App>,
    pub(crate) windows: Vec<WindowEntry>,
    /// Contenu de la Corbeille (voir Mode::RecycleBin) -- `mode_items` n'en
    /// dérive que les noms ; gardé à part, comme `windows`, pour retrouver
    /// le chemin `$I`/`$R` réel d'un élément sélectionné.
    pub(crate) recycle_bin_items: Vec<RecycleBinItem>,
    /// `Some` tant qu'un scan Corbeille lancé sur un thread dédié (voir
    /// rebuild_recycle_bin_items) n'a pas rendu son résultat -- sondé par
    /// RECYCLE_BIN_POLL_TIMER_ID. `list_items()` énumère TOUS les lecteurs
    /// sur disque : l'appeler sur le thread UI gèlerait le lanceur le temps
    /// du scan, même raison que empty_async() côté vidage.
    pub(crate) recycle_bin_pending: Option<std::sync::mpsc::Receiver<Vec<RecycleBinItem>>>,
    /// Contenu du mode Eject -- même rôle que `windows` pour le Window
    /// Switcher : `mode_items` n'en dérive que le libellé, la lettre de
    /// lecteur réelle reste ici (voir launch_selected).
    pub(crate) eject_drives: Vec<EjectableDrive>,
    /// `None` si emoji-test.txt est absent/illisible à côté de l'exe (voir
    /// core::emoji::load) -- le mode Emoji reste alors inatteignable (voir
    /// launch_selected_normal) plutôt que d'ouvrir un picker vide.
    pub(crate) emoji: Option<EmojiData>,
    pub(crate) notes: Vec<String>,
    pub(crate) notes_path: PathBuf,
    pub(crate) restart_targets: Vec<String>,
    pub(crate) restart_path: PathBuf,
    pub(crate) kill_targets: Vec<String>,
    pub(crate) kill_path: PathBuf,
    /// Superviseur unique Auto-restart + Auto-kill (voir core::supervisor) --
    /// un seul thread de sondage pour les deux.
    pub(crate) process_supervisor: ProcessSupervisor,
    /// Reflètent quelle facette du superviseur est active -- exposés au tray
    /// (voir "Disable Auto-restart" / "Disable Auto-kill").
    pub(crate) auto_restart_enabled: bool,
    pub(crate) auto_kill_enabled: bool,

    /// RAM-only, jamais écrit sur disque -- voir core::clipboard_history
    /// pour le détail de sécurité (VirtualLock + effacement à zéro, sans
    /// chiffrement). Alimenté par WM_CLIPBOARDUPDATE tant que
    /// `copy_history_enabled` est actif.
    pub(crate) copy_history: ClipboardHistory,
    /// Reflète si le listener presse-papier (AddClipboardFormatListener)
    /// est enregistré (voir core::state::State::copy_history_enabled).
    pub(crate) copy_history_enabled: bool,
    /// Positionné juste avant un `set_clipboard_text` émis par le lanceur
    /// lui-même (re-copie d'une entrée de l'historique) et consommé par le
    /// prochain WM_CLIPBOARDUPDATE, qui est alors ignoré : sans ça, la
    /// re-copie réinjecterait un doublon en tête de l'historique.
    pub(crate) suppress_next_clipboard_capture: bool,

    pub(crate) mode: Mode,
    /// Libellés actuellement filtrables/affichables -- reflète `apps` en
    /// mode Normal, `windows`/`notes`/`restart_targets`/les noms de thème
    /// dans les autres modes. Un seul chemin de filtrage/rendu pour tous
    /// les modes (voir le commentaire en tête de fichier).
    pub(crate) mode_items: Vec<String>,
    /// `(dernier mode_items vu, sa version normalisée)` -- accents repliés
    /// et minuscules, voir `normalized_mode_items`. `fuzzy_filter` tourne à
    /// chaque frappe alors que `mode_items` ne change qu'à l'entrée d'un
    /// mode ou après un ajout/suppression : renormaliser plusieurs milliers
    /// d'entrées (catalogue Emoji, une String allouée chacune) à chaque
    /// frappe serait du travail perdu. Invalidation par comparaison
    /// d'égalité, donc jamais désynchronisé -- aucun compteur à incrémenter
    /// sur les nombreux sites qui réaffectent `mode_items`.
    pub(crate) mode_items_cache: (Vec<String>, Vec<String>),
    pub(crate) filtered: Vec<usize>,
    pub(crate) selected: usize,
    pub(crate) first_visible: usize,
    pub(crate) display: SearchDisplay,

    pub(crate) theme: ThemeConfig,
    pub(crate) theme_picker_original: Option<String>,
    /// `false` si themes.json est absent/invalide/sans thème exploitable
    /// (voir theme::load) : `theme.current` retombe sur le thème de secours
    /// codé en dur. Même principe que `emoji` pour emoji-test.txt -- rendu
    /// explicite dans row_label ("Theme: missing themes.json") et bloque
    /// l'entrée dans le sélecteur, plutôt que de masquer le problème
    /// derrière le thème de secours.
    pub(crate) themes_json_present: bool,
    pub(crate) base_dir: PathBuf,
    pub(crate) themes_path: PathBuf,
    /// state.json -- thème actif/police/window_size/border et les bascules
    /// auto-restart/copy-history (voir core::state), commité à chaque
    /// changement (theme picker, Ctrl+1..9/0, Ctrl+-/=, menu tray).
    pub(crate) state_path: PathBuf,
    /// Réenregistre le hotkey global sur la fenêtre TRAY (voir ui::tray) --
    /// posé par main.rs juste après la création du tray, `None` avant ça
    /// (fenêtre de temps où aucun Reload ne peut de toute façon survenir).
    /// Ce module ne connaît ni `TrayState` ni le hwnd du tray directement :
    /// le hotkey vit entièrement côté main.rs/ui::tray, ce hook est le seul
    /// pont entre les deux, appelé par `reload_config` avec le hotkey/
    /// hotkey_enabled frais lus de state.json.
    pub(crate) on_hotkey_reload: Option<Box<dyn FnMut(&str, bool)>>,

    // Timer + rebond DVD
    pub(crate) timer_deadline: Option<Instant>,
    pub(crate) bouncing: bool,
    pub(crate) bounce_pos: (f64, f64),
    pub(crate) bounce_vel: (f64, f64),
    pub(crate) bounce_pre_geometry: Option<Geometry>,
    pub(crate) bounce_pre_theme: Option<String>,
    pub(crate) rng: SimpleRng,

    pub(crate) edit_hwnd: HWND,
    pub(crate) geometry: Geometry,
    pub(crate) font_row: HFONT,
    pub(crate) font_search: HFONT,
    /// Famille/taille (px) actuellement appliquées à font_row/font_search --
    /// comparées à chaque appel de apply_theme_visuals() pour ne recréer
    /// les polices que si l'une des deux a changé, et pas à chaque
    /// changement de thème couleur.
    pub(crate) applied_font_family: String,
    pub(crate) applied_font_px: i32,
    pub(crate) search_brush: HBRUSH,
    /// Pinceaux de thème mis en cache -- créés une fois par changement de
    /// thème (apply_theme_visuals), jamais dans draw_scene. Sans ce cache,
    /// chaque case de la liste crée puis détruit son propre pinceau à
    /// CHAQUE repaint (jusqu'à ~14 paires CreateSolidBrush/DeleteObject par
    /// frame), coût payé en plein sur un défilement continu.
    pub(crate) list_bg_brush: HBRUSH,
    pub(crate) selected_bg_brush: HBRUSH,
    pub(crate) border_brush: HBRUSH,
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
    pub(crate) mem_dc: HDC,
    pub(crate) mem_bitmap: HBITMAP,
    /// Dimensions pour lesquelles mem_dc/mem_bitmap ont été créés -- pour ne
    /// les recréer QUE si la taille a réellement changé (voir
    /// ensure_scene_buffer), même principe que applied_font_family/px.
    pub(crate) mem_buffer_size: (i32, i32),
    /// Texte d'invite, dessiné à la main dans edit_subclass_proc -- gardé
    /// en UTF-16 prêt pour DrawTextW plutôt que reconverti à chaque repaint.
    pub(crate) placeholder_wide: Vec<u16>,
    /// Marge fixée sur edit_hwnd via EM_SETMARGINS (voir
    /// apply_theme_visuals), réutilisée telle quelle par draw_placeholder et
    /// par l'horloge (draw_clock_text) : même marge générale que tout texte
    /// respecte (voir text_margin_px).
    pub(crate) text_margin: i32,
    /// Cache de `recycle_bin::query()` (compte + taille). Un Cell, donc
    /// modifiable à travers un &AppState partagé (voir row_label) :
    /// SHQueryRecycleBinW touche le disque et peut prendre plusieurs
    /// dizaines de ms, assez pour bloquer la pompe de messages une frame à
    /// chaque frappe si la ligne Corbeille est visible.
    pub(crate) recycle_bin_cache: std::cell::Cell<Option<(Instant, i64, i64)>>,
}

/// Durée de validité du cache ci-dessus -- assez court pour qu'un vidage de
/// Corbeille (magi:empty-recycle-bin) se reflète vite, assez long pour
/// qu'une rafale de repaints (navigation clavier, rebond DVD) ne déclenche
/// qu'une requête réelle plutôt qu'une par frame.
const RECYCLE_BIN_CACHE_TTL: Duration = Duration::from_secs(2);

pub(crate) fn recycle_bin_cached(state: &AppState) -> (i64, i64) {
    if let Some((at, count, size)) = state.recycle_bin_cache.get() {
        if at.elapsed() < RECYCLE_BIN_CACHE_TTL {
            return (count, size);
        }
    }
    let (count, size) = crate::core::recycle_bin::query();
    state.recycle_bin_cache.set(Some((Instant::now(), count, size)));
    (count, size)
}

/// Recalcule `placeholder_wide` selon le mode courant. Appelée par
/// apply_theme_visuals (un changement de thème ne doit pas écraser un
/// placeholder de mode par le générique) ET par enter_mode (un changement
/// de mode seul doit aussi mettre à jour le texte affiché).
pub(crate) fn refresh_placeholder(state: &mut AppState) {
    let text = match state.mode {
        Mode::Timer => TIMER_PLACEHOLDER,
        Mode::Notes => NOTES_PLACEHOLDER,
        Mode::Restart => RESTART_PLACEHOLDER,
        Mode::Kill => KILL_PLACEHOLDER,
        _ => state.theme.placeholder_text.as_str(),
    };
    state.placeholder_wide = to_wstring(text);
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

/// Reconstruit le look de la fenêtre (polices, pinceaux, marges,
/// placeholder) à partir de `state.theme.current` -- appelée à la création
/// et à chaque changement de thème. Deux polices distinctes : la recherche
/// reste légèrement plus grande que les lignes de résultat.
pub(crate) unsafe fn apply_theme_visuals(state: &mut AppState) {
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

pub(crate) fn get_edit_text(edit_hwnd: HWND) -> String {
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

pub(crate) fn set_edit_text(edit_hwnd: HWND, text: &str) {
    unsafe {
        SetWindowTextW(edit_hwnd, to_wstring(text).as_ptr());
    }
}
