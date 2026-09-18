//! Chargement/validation/preview des palettes (themes.json) + application
//! des préférences d'affichage persistées dans state.json (voir
//! `apply_prefs`, core::state::UiPrefs). L'état vit dans une struct
//! `ThemeConfig` normale possédée par l'état de la fenêtre.
//!
//! Validation des couleurs : themes.json n'utilise jamais que des hex
//! `#rrggbb`/`#rgb` (voir le format documenté dans le README), donc un
//! parseur hex direct suffit, pas besoin d'un moteur de couleurs complet.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::json::Json;
use crate::win32::gdi32::rgb;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub search_background: u32,
    pub search_text: u32,
    pub list_background: u32,
    pub list_text: u32,
    pub selected_background: u32,
    pub selected_text: u32,
    pub border: u32,
}

impl Theme {
    /// Thème de repli codé en dur (couleurs de "arc-dark"), utilisé si
    /// themes.json est absent ou invalide. Jamais écrit sur disque.
    fn fallback() -> Theme {
        Theme {
            search_background: rgb(0x40, 0x45, 0x52),
            search_text: rgb(0x7c, 0x81, 0x8c),
            list_background: rgb(0x38, 0x3c, 0x4a),
            list_text: rgb(0xd3, 0xda, 0xe3),
            selected_background: rgb(0x52, 0x94, 0xe2),
            selected_text: rgb(0xff, 0xff, 0xff),
            border: rgb(0x4b, 0x51, 0x62),
        }
    }
}

pub struct ThemeConfig {
    /// Nom du thème persisté dans state.json (voir apply_prefs) -- distinct
    /// de `current`, qui peut diverger temporairement pendant une preview
    /// (voir preview_theme) sans jamais toucher au fichier ni à ce champ.
    pub active_theme: String,
    pub font_family: Option<String>,
    pub placeholder_text: String,
    pub show_clock: bool,
    /// Fraction 0.0-1.0 de la largeur d'écran, la forme qu'attend toute la
    /// géométrie de ui::window. Persisté dans state.json sous "window_size"
    /// comme entier 0-100 (%), plus lisible à l'édition manuelle : seule
    /// `apply_prefs` connaît ce facteur 100.
    pub window_width_fraction: f64,
    pub border_width: i32,
    pub themes: HashMap<String, Theme>,
    pub current: Theme,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let mut themes = HashMap::new();
        themes.insert("arc-dark".to_string(), Theme::fallback());
        ThemeConfig {
            active_theme: "arc-dark".to_string(),
            font_family: None,
            placeholder_text: "Type to search...".to_string(),
            show_clock: true,
            window_width_fraction: 0.30,
            border_width: 3,
            current: Theme::fallback(),
            themes,
        }
    }
}

/// "#rrggbb" ou "#rgb" -> COLORREF (0x00bbggrr, l'ordre attendu par GDI,
/// voir gdi32::rgb). `None` si la chaîne n'est pas un hex valide. Publique
/// car ui::window s'en sert aussi pour l'aperçu couleur inline de la
/// recherche, qui applique la même règle de reconnaissance.
pub fn parse_hex_color(s: &str) -> Option<u32> {
    let s = s.strip_prefix('#')?;
    let (r, g, b) = match s.len() {
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
        ),
        3 => {
            let mut chars = s.chars();
            let expand = |c: char| -> Option<u8> {
                let d = c.to_digit(16)? as u8;
                Some(d * 16 + d)
            };
            (expand(chars.next()?)?, expand(chars.next()?)?, expand(chars.next()?)?)
        }
        _ => return None,
    };
    Some(rgb(r, g, b))
}

fn parse_theme_entry(v: &Json) -> Option<Theme> {
    let color = |key: &str| v.get(key).and_then(Json::as_str).and_then(parse_hex_color);
    Some(Theme {
        search_background: color("search_background")?,
        search_text: color("search_text")?,
        list_background: color("list_background")?,
        list_text: color("list_text")?,
        selected_background: color("selected_background")?,
        selected_text: color("selected_text")?,
        border: color("border")?,
    })
}

/// Charge le catalogue `themes` de themes.json dans `cfg`. Silencieux sur
/// toute erreur (fichier absent, JSON invalide, aucun thème exploitable) :
/// `cfg` conserve alors son état précédent, ou les valeurs d'usine au
/// premier appel. Retourne `false` dans ces cas, pour que l'appelant puisse
/// le signaler plutôt que de masquer le problème derrière le thème de
/// secours. Le thème ACTIF et les autres préférences d'affichage vivent
/// dans state.json désormais (voir `apply_prefs`, appelée juste après par
/// tous les appelants) -- themes.json ne porte plus que les palettes.
pub fn load(path: &Path, cfg: &mut ThemeConfig) -> bool {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let data = match Json::parse(&text) {
        Ok(d) => d,
        Err(_) => return false,
    };
    let obj = match data.as_object() {
        Some(o) => o,
        None => return false,
    };

    let mut themes = HashMap::new();
    if let Some(theme_obj) = obj.get("themes").and_then(Json::as_object) {
        for (name, v) in theme_obj {
            if let Some(t) = parse_theme_entry(v) {
                themes.insert(name.clone(), t);
            }
        }
    }
    if themes.is_empty() {
        return false;
    }
    cfg.themes = themes;
    true
}

/// Applique les préférences d'affichage persistées dans state.json (voir
/// core::state::UiPrefs) à `cfg`, et résout `current` contre le catalogue
/// déjà chargé par `load` -- à appeler juste après `load`, à chaque
/// (re)chargement (démarrage, Reload). Bornées, jamais reprises telles
/// quelles : une valeur absurde (infinie, négative, NaN) aurait déjà été
/// écartée par `core::state::load`, mais `window_size`/`border` restent
/// clampées ici aussi, ce module ne faisant pas confiance à l'appelant pour
/// avoir déjà validé ce qu'il transmet.
pub fn apply_prefs(cfg: &mut ThemeConfig, prefs: &crate::core::state::UiPrefs) {
    let applied = cfg.themes.get(&prefs.theme).copied().unwrap_or_else(|| *cfg.themes.values().next().unwrap_or(&Theme::fallback()));
    cfg.active_theme = prefs.theme.clone();
    cfg.font_family = prefs.font_family.clone();
    cfg.placeholder_text = prefs.placeholder_text.clone();
    cfg.show_clock = prefs.show_clock;
    // "window_size" est un pourcentage 0-100 côté state.json, converti en
    // fraction ici (voir le champ window_width_fraction).
    cfg.window_width_fraction = (prefs.window_size as f64 / 100.0).clamp(0.05, 1.0);
    cfg.border_width = prefs.border.clamp(0, 100);
    cfg.current = applied;
}

/// Applique les couleurs d'un thème par son nom, sans toucher au disque ni
/// à `active_theme` : la preview en direct du sélecteur de thème (voir
/// ui::window), appelée à chaque déplacement de la sélection clavier. Ne
/// fait rien si le nom est inconnu.
pub fn preview_theme(cfg: &mut ThemeConfig, name: &str) {
    if let Some(t) = cfg.themes.get(name) {
        cfg.current = *t;
    }
}

/// Noms de thèmes triés alphabétiquement, pour peupler le sélecteur.
/// L'ordre d'insertion de themes.json n'est pas restituable (le parseur
/// stocke les objets en HashMap) ; seul un ordre déterministe importe ici.
pub fn list_theme_names(cfg: &ThemeConfig) -> Vec<String> {
    let mut names: Vec<String> = cfg.themes.keys().cloned().collect();
    names.sort();
    names
}

/// Famille de police à utiliser : celle de state.json si réglée, sinon
/// Segoe UI, la police système par défaut de Windows depuis Vista.
pub fn resolve_font_family(cfg: &ThemeConfig) -> String {
    cfg.font_family.clone().unwrap_or_else(|| "Segoe UI".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_theme_test_{}_{}.json", std::process::id(), name));
        p
    }

    const SAMPLE: &str = r##"{
  "themes": {
    "night": {
      "search_background": "#404552",
      "search_text": "#7c818c",
      "list_background": "#383c4a",
      "list_text": "#d3dae3",
      "selected_background": "#5294e2",
      "selected_text": "#ffffff",
      "border": "#4b5162"
    },
    "day": {
      "search_background": "#ffffff",
      "search_text": "#000000",
      "list_background": "#eeeeee",
      "list_text": "#111111",
      "selected_background": "#3366cc",
      "selected_text": "#ffffff",
      "border": "#cccccc"
    }
  }
}"##;

    #[test]
    fn parse_hex_color_couvre_3_et_6_chiffres() {
        assert_eq!(parse_hex_color("#fff"), Some(rgb(255, 255, 255)));
        assert_eq!(parse_hex_color("#3a8ea0"), Some(rgb(0x3a, 0x8e, 0xa0)));
        assert_eq!(parse_hex_color("bogus"), None);
        assert_eq!(parse_hex_color("#12"), None);
    }

    #[test]
    fn charge_le_catalogue_de_themes() {
        let path = temp_path("load_ok");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        assert!(load(&path, &mut cfg));
        assert_eq!(cfg.themes.len(), 2);
        assert_eq!(cfg.themes.get("night").unwrap().search_background, rgb(0x40, 0x45, 0x52));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn fichier_absent_garde_l_etat_precedent() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        let mut cfg = ThemeConfig::default();
        let before = cfg.current;
        assert!(!load(&path, &mut cfg));
        assert_eq!(cfg.current, before);
    }

    fn sample_prefs(theme: &str) -> crate::core::state::UiPrefs {
        crate::core::state::UiPrefs {
            theme: theme.to_string(),
            font_family: Some("Segoe UI".to_string()),
            placeholder_text: "Type to search...".to_string(),
            show_clock: true,
            window_size: 30,
            border: 3,
        }
    }

    #[test]
    fn apply_prefs_resout_le_theme_actif_et_les_reglages() {
        let path = temp_path("apply_prefs");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        load(&path, &mut cfg);
        apply_prefs(&mut cfg, &sample_prefs("night"));
        assert_eq!(cfg.active_theme, "night");
        assert_eq!(cfg.current.search_background, rgb(0x40, 0x45, 0x52));
        assert_eq!(cfg.font_family.as_deref(), Some("Segoe UI"));
        assert_eq!(cfg.border_width, 3);
        assert_eq!(cfg.window_width_fraction, 0.30);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn apply_prefs_replie_sur_le_premier_theme_si_le_nom_est_inconnu() {
        let path = temp_path("apply_prefs_unknown");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        load(&path, &mut cfg);
        apply_prefs(&mut cfg, &sample_prefs("does-not-exist"));
        assert!(cfg.themes.values().any(|t| *t == cfg.current));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn preview_change_current_sans_toucher_active_theme() {
        let path = temp_path("preview");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        load(&path, &mut cfg);
        apply_prefs(&mut cfg, &sample_prefs("night"));
        preview_theme(&mut cfg, "day");
        assert_eq!(cfg.current.search_background, rgb(0xff, 0xff, 0xff));
        assert_eq!(cfg.active_theme, "night"); // inchangé
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn list_theme_names_est_triee() {
        let path = temp_path("list_names");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        load(&path, &mut cfg);
        assert_eq!(list_theme_names(&cfg), vec!["day".to_string(), "night".to_string()]);
        let _ = fs::remove_file(&path);
    }
}
