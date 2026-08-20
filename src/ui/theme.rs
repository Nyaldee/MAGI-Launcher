//! Chargement/validation/preview/commit des thèmes (themes.json). L'état
//! vit dans une struct `ThemeConfig` normale possédée par l'état de la
//! fenêtre.
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
    /// Nom du thème persisté dans themes.json -- distinct de `current`,
    /// qui peut diverger temporairement pendant une preview (voir
    /// preview_theme) sans jamais toucher au fichier ni à ce champ.
    pub active_theme: String,
    pub font_family: Option<String>,
    pub placeholder_text: String,
    pub show_clock: bool,
    /// Fraction 0.0-1.0 de la largeur d'écran, la forme qu'attend toute la
    /// géométrie de ui::window. Persisté dans themes.json sous "window_size"
    /// comme entier 0-100 (%), plus lisible à l'édition manuelle : seuls
    /// `load` et `commit_window_size` connaissent ce facteur 100.
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

/// Charge themes.json dans `cfg`. Silencieux sur toute erreur (fichier
/// absent, JSON invalide, aucun thème exploitable) : `cfg` conserve alors
/// son état précédent, ou les valeurs d'usine au premier appel. Retourne
/// `false` dans ces cas, pour que l'appelant puisse le signaler plutôt que
/// de masquer le problème derrière le thème de secours.
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

    let active = obj.get("theme").and_then(Json::as_str).unwrap_or(&cfg.active_theme).to_string();
    let applied = themes.get(&active).copied().unwrap_or_else(|| *themes.values().next().unwrap());

    cfg.font_family = obj.get("font_family").and_then(Json::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    cfg.placeholder_text =
        obj.get("placeholder_text").and_then(Json::as_str).unwrap_or("Type to search...").to_string();
    cfg.show_clock = obj.get("show_clock").and_then(Json::as_bool).unwrap_or(true);
    // Bornées, jamais reprises telles quelles : une valeur JSON valide mais
    // absurde (infinie, négative, NaN) atteindrait compute_geometry, où
    // `2 * border` peut alors déborder i32 et abattre l'appli. Hors de ces
    // bornes, la valeur n'a de toute façon aucun sens visuel.
    // "window_size" est un pourcentage 0-100 côté JSON, converti en fraction
    // ici (voir le champ window_width_fraction).
    cfg.window_width_fraction = obj
        .get("window_size")
        .and_then(Json::as_f64)
        .filter(|n| n.is_finite())
        .map(|n| (n / 100.0).clamp(0.05, 1.0))
        .unwrap_or(0.30);
    cfg.border_width =
        obj.get("border").and_then(Json::as_f64).filter(|n| n.is_finite()).map(|n| (n as i32).clamp(0, 100)).unwrap_or(3);
    cfg.active_theme = active;
    cfg.current = applied;
    cfg.themes = themes;
    true
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

/// Écrit le nouveau thème actif dans themes.json en remplaçant la seule
/// valeur de la clé "theme", sans resérialiser le document : le formatage
/// édité à la main est ainsi préservé.
pub fn commit_theme(path: &Path, new_name: &str) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let quote_pos = crate::json::locate_value_start(&text, "theme")?;
    // Type vérifié, pas supposé : si "theme" porte autre chose qu'une chaîne
    // (fichier édité à la main, ex. `"theme": 5`), chercher le prochain '"'
    // à l'aveugle tomberait sur un guillemet bien plus loin -- typiquement
    // la clé suivante -- et écraserait tout ce qui les sépare.
    if !text[quote_pos..].starts_with('"') {
        return Err("valeur de 'theme' n'est pas une chaîne".to_string());
    }
    let value_start = quote_pos + 1;
    let value_end =
        text[value_start..].find('"').map(|i| value_start + i).ok_or("valeur de 'theme' non terminée")?;

    let mut new_text = String::with_capacity(text.len());
    new_text.push_str(&text[..value_start]);
    // Échappé, pas inséré tel quel : `new_name` vient des clés de l'objet
    // "themes" du même fichier, et rien n'interdit à une clé écrite à la
    // main de contenir un '"' ou un '\\', ce qui casserait le JSON produit.
    crate::json::escape_json_content(new_name, &mut new_text);
    new_text.push_str(&text[value_end..]);
    fs::write(path, new_text).map_err(|e| e.to_string())
}

/// Remplace la valeur numérique associée à `key` à la racine de themes.json,
/// par le même remplacement ciblé que `commit_theme`. Une valeur numérique
/// n'a pas de délimiteur explicite : sa fin est le premier caractère
/// n'appartenant plus à un littéral nombre (chiffre, signe, point, exposant).
fn commit_number(path: &Path, key: &str, value: i64) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value_start = crate::json::locate_value_start(&text, key)?;
    let value_len = text[value_start..]
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E'))
        .unwrap_or(text.len() - value_start);
    let value_end = value_start + value_len;

    let mut new_text = String::with_capacity(text.len());
    new_text.push_str(&text[..value_start]);
    new_text.push_str(&value.to_string());
    new_text.push_str(&text[value_end..]);
    fs::write(path, new_text).map_err(|e| e.to_string())
}

/// Persiste la taille de fenêtre (voir Ctrl+1..9/0 dans ui::window) --
/// `size_percent` est un pourcentage 0-100, la représentation JSON de
/// "window_size" (voir `load`).
pub fn commit_window_size(path: &Path, size_percent: i32) -> Result<(), String> {
    commit_number(path, "window_size", size_percent as i64)
}

/// Persiste l'épaisseur de bordure (voir Ctrl+-/Ctrl+= dans ui::window).
pub fn commit_border(path: &Path, border_px: i32) -> Result<(), String> {
    commit_number(path, "border", border_px as i64)
}

/// Famille de police à utiliser : celle de themes.json si réglée, sinon
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
  "theme": "night",
  "font_family": "Segoe UI",
  "placeholder_text": "Type to search...",
  "show_clock": true,
  "window_size": 30,
  "border": 3,
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
    fn charge_le_theme_actif_et_les_reglages_racine() {
        let path = temp_path("load_ok");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        assert!(load(&path, &mut cfg));
        assert_eq!(cfg.active_theme, "night");
        assert_eq!(cfg.current.search_background, rgb(0x40, 0x45, 0x52));
        assert_eq!(cfg.font_family.as_deref(), Some("Segoe UI"));
        assert_eq!(cfg.border_width, 3);
        assert_eq!(cfg.window_width_fraction, 0.30);
        assert_eq!(cfg.themes.len(), 2);
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

    #[test]
    fn preview_change_current_sans_toucher_active_theme() {
        let path = temp_path("preview");
        fs::write(&path, SAMPLE).unwrap();
        let mut cfg = ThemeConfig::default();
        load(&path, &mut cfg);
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

    #[test]
    fn commit_theme_remplace_seulement_la_valeur_theme() {
        let path = temp_path("commit");
        fs::write(&path, SAMPLE).unwrap();
        commit_theme(&path, "day").unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("\"theme\": \"day\""));
        // Le reste du fichier (formatage, autres clés) doit rester intact.
        assert!(after.contains("\"font_family\": \"Segoe UI\""));
        assert!(after.contains("\"night\": {"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_theme_echappe_un_nom_avec_guillemet_ou_antislash() {
        let path = temp_path("commit_escape");
        fs::write(&path, SAMPLE).unwrap();
        let tricky = "a\"b\\c";
        commit_theme(&path, tricky).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        // Le fichier écrit doit rester du JSON valide, et se relire vers le
        // même nom en clair (aller-retour intact).
        let parsed = crate::json::Json::parse(&after).unwrap();
        assert_eq!(parsed.get("theme").unwrap().as_str(), Some(tricky));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_window_size_remplace_seulement_cette_valeur() {
        let path = temp_path("commit_window_size");
        fs::write(&path, SAMPLE).unwrap();
        commit_window_size(&path, 90).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("\"window_size\": 90"));
        assert!(after.contains("\"border\": 3")); // reste intact
        let mut cfg = ThemeConfig::default();
        assert!(load(&path, &mut cfg));
        assert_eq!(cfg.window_width_fraction, 0.90);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_border_remplace_seulement_cette_valeur() {
        let path = temp_path("commit_border");
        fs::write(&path, SAMPLE).unwrap();
        commit_border(&path, 7).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("\"border\": 7"));
        assert!(after.contains("\"window_size\": 30")); // reste intact
        let mut cfg = ThemeConfig::default();
        assert!(load(&path, &mut cfg));
        assert_eq!(cfg.border_width, 7);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_number_gere_les_valeurs_negatives() {
        let path = temp_path("commit_negative");
        fs::write(&path, SAMPLE).unwrap();
        commit_border(&path, -1).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("\"border\": -1"));
        let _ = fs::remove_file(&path);
    }
}
