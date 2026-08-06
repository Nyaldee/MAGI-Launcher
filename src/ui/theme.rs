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
    /// Thème de repli codé en dur -- utilisé si themes.json est absent ou
    /// entièrement invalide, jamais écrit sur disque. Mêmes couleurs que
    /// l'entrée "arc-dark" par défaut de l'original.
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

/// "#rrggbb" ou "#rgb" -> COLORREF (0x00bbggrr, l'ordre attendu par GDI --
/// voir gdi32::rgb). `None` si la chaîne n'est pas un hex valide. Publique
/// : aussi réutilisée par ui::window pour l'aperçu couleur inline de la
/// recherche (même règle de reconnaissance qu'un thème, pas de raison de
/// dupliquer le parseur).
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

/// Charge themes.json dans `cfg`, en silence sur toute erreur (fichier
/// absent, JSON invalide, aucun thème exploitable) -- `cfg` garde
/// simplement son état précédent (ou les valeurs par défaut d'usine si
/// c'est le tout premier appel), jamais de plantage pour un fichier de
/// thème mal formé.
pub fn load(path: &Path, cfg: &mut ThemeConfig) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };
    let data = match Json::parse(&text) {
        Ok(d) => d,
        Err(_) => return,
    };
    let obj = match data.as_object() {
        Some(o) => o,
        None => return,
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
        return;
    }

    let active = obj.get("theme").and_then(Json::as_str).unwrap_or(&cfg.active_theme).to_string();
    let applied = themes.get(&active).copied().unwrap_or_else(|| *themes.values().next().unwrap());

    cfg.font_family = obj.get("font_family").and_then(Json::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    cfg.placeholder_text =
        obj.get("placeholder_text").and_then(Json::as_str).unwrap_or("Type to search...").to_string();
    cfg.show_clock = obj.get("show_clock").and_then(Json::as_bool).unwrap_or(true);
    // Bornées plutôt qu'utilisées telles quelles : une valeur JSON
    // techniquement valide mais absurde (infinie, négative, NaN via un
    // exposant énorme ou une faute de frappe) se propageait telle quelle
    // jusqu'au calcul de géométrie (compute_geometry), où `2 * border`
    // pouvait dépasser i32 et planter -- une fraction de fenêtre ou une
    // bordure hors de ces bornes n'a de toute façon aucun sens visuel.
    cfg.window_width_fraction = obj
        .get("window_width_fraction")
        .and_then(Json::as_f64)
        .filter(|n| n.is_finite())
        .map(|n| n.clamp(0.05, 1.0))
        .unwrap_or(0.30);
    cfg.border_width =
        obj.get("border").and_then(Json::as_f64).filter(|n| n.is_finite()).map(|n| (n as i32).clamp(0, 100)).unwrap_or(3);
    cfg.active_theme = active;
    cfg.current = applied;
    cfg.themes = themes;
}

/// Applique les couleurs d'un thème par son nom SANS toucher au disque ni
/// à `active_theme` -- le mécanisme de preview en direct du sélecteur de
/// thème (voir ui::window), appelé à chaque déplacement de la sélection
/// clavier. Ne fait rien si le nom est inconnu.
pub fn preview_theme(cfg: &mut ThemeConfig, name: &str) {
    if let Some(t) = cfg.themes.get(name) {
        cfg.current = *t;
    }
}

/// Noms de thèmes triés, pour peupler le sélecteur -- alphabétique plutôt
/// que l'ordre d'insertion de themes.json (que notre parseur JSON, basé
/// sur une HashMap, ne préserve de toute façon pas) : simplification
/// délibérée, l'ordre de présentation des thèmes n'a pas d'importance
/// fonctionnelle, seul le fait qu'il soit stable et déterministe compte.
pub fn list_theme_names(cfg: &ThemeConfig) -> Vec<String> {
    let mut names: Vec<String> = cfg.themes.keys().cloned().collect();
    names.sort();
    names
}

/// Écrit le nouveau thème actif dans themes.json via un remplacement de
/// sous-chaîne ciblé sur la valeur de la clé "theme" -- pas une
/// resérialisation complète -- pour préserver le formatage édité à la
/// main du fichier.
pub fn commit_theme(path: &Path, new_name: &str) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let key_pos = text.find("\"theme\"").ok_or("clé 'theme' introuvable dans themes.json")?;
    let after_key = &text[key_pos + 7..];
    let colon_rel = after_key.find(':').ok_or("':' manquant après 'theme'")?;
    let after_colon = &after_key[colon_rel + 1..];
    let quote_start_rel = after_colon.find('"').ok_or("valeur de 'theme' introuvable")?;
    let value_start = key_pos + 7 + colon_rel + 1 + quote_start_rel + 1;
    let value_end =
        text[value_start..].find('"').map(|i| value_start + i).ok_or("valeur de 'theme' non terminée")?;

    let mut new_text = String::with_capacity(text.len());
    new_text.push_str(&text[..value_start]);
    // Échappé, pas inséré tel quel : `new_name` vient des clés de l'objet
    // "themes" du même fichier (voir list_theme_names), donc en théorie
    // déjà de simples identifiants -- mais rien n'empêche un thème édité à
    // la main d'avoir une clé contenant un '"' ou un '\\', ce qui casserait
    // silencieusement le JSON écrit ici sans cet échappement.
    crate::json::escape_json_content(new_name, &mut new_text);
    new_text.push_str(&text[value_end..]);
    fs::write(path, new_text).map_err(|e| e.to_string())
}

/// Famille de police à utiliser -- celle de themes.json si réglée, sinon
/// Segoe UI (police système par défaut de Windows depuis Vista, ce que
/// l'original obtenait indirectement via la police par défaut de Tk).
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
  "window_width_fraction": 0.30,
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
        load(&path, &mut cfg);
        assert_eq!(cfg.active_theme, "night");
        assert_eq!(cfg.current.search_background, rgb(0x40, 0x45, 0x52));
        assert_eq!(cfg.font_family.as_deref(), Some("Segoe UI"));
        assert_eq!(cfg.border_width, 3);
        assert_eq!(cfg.themes.len(), 2);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn fichier_absent_garde_l_etat_precedent() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        let mut cfg = ThemeConfig::default();
        let before = cfg.current;
        load(&path, &mut cfg);
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
}
