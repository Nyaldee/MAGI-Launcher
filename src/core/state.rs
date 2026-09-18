//! Bascules persistées du lanceur (state.json) -- séparées de apps.json
//! (catalogue) et themes.json (palettes), qui restent tous deux éditables à
//! la main sans jamais contenir de préférence d'affichage/comportement.
//! Contrairement à ces deux-là, state.json est un fichier interne : réécrit
//! EN ENTIER à chaque changement (voir `save`), pas de remplacement ciblé
//! préservant un formatage édité à la main.

use std::fs;
use std::path::Path;

use crate::json::{escape_json_content, Json};

pub struct UiPrefs {
    pub theme: String,
    pub font_family: Option<String>,
    pub placeholder_text: String,
    pub show_clock: bool,
    /// Pourcentage 0-100, la même représentation que "window_size" sur
    /// disque -- la conversion en fraction 0.0-1.0 attendue par
    /// `ui::theme::ThemeConfig`/la géométrie reste le problème de
    /// l'appelant (voir `ui::theme::apply_prefs`), pas de ce module.
    pub window_size: i32,
    pub border: i32,
}

impl Default for UiPrefs {
    fn default() -> Self {
        UiPrefs {
            theme: "arc-dark".to_string(),
            font_family: None,
            placeholder_text: "Type to search...".to_string(),
            show_clock: true,
            window_size: 30,
            border: 1,
        }
    }
}

pub struct State {
    pub ui: UiPrefs,
    pub auto_restart_enabled: bool,
    /// Auto-kill (voir core::supervisor). Défaut `false`, contrairement à
    /// `auto_restart_enabled` : une fonction qui TERMINE des processus doit
    /// être opt-in explicite -- personne ne se retrouve avec un tueur armé
    /// après une mise à jour.
    pub auto_kill_enabled: bool,
    pub copy_history_enabled: bool,
    pub hotkey_enabled: bool,
    pub hotkey: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            ui: UiPrefs::default(),
            auto_restart_enabled: true,
            auto_kill_enabled: false,
            copy_history_enabled: true,
            hotkey_enabled: true,
            hotkey: "Ctrl+Space".to_string(),
        }
    }
}

/// Fichier absent/invalide/partiellement rempli -- jamais fatal, contrairement
/// à apps.json : state.json n'est qu'un ensemble de préférences, un champ
/// manquant ou tout le fichier absent (premier lancement) retombe simplement
/// sur sa valeur par défaut individuelle, jamais sur un blocage au démarrage.
/// Sauvegarde toujours le résultat avant de le renvoyer -- même principe que
/// `StateManager::load` côté Ports Launcher : le fichier existe ainsi sur
/// disque dès la fin du tout premier lancement, prêt à être inspecté/édité
/// pour le suivant, plutôt que de n'apparaître qu'au premier `commit_*`.
pub fn load(path: &Path) -> State {
    let state = parse(path);
    let _ = save(path, &state);
    state
}

fn parse(path: &Path) -> State {
    let mut state = State::default();
    let Ok(text) = fs::read_to_string(path) else { return state };
    let Ok(data) = Json::parse(&text) else { return state };
    let Some(obj) = data.as_object() else { return state };

    if let Some(ui) = obj.get("ui").and_then(Json::as_object) {
        state.ui.theme = ui.get("theme").and_then(Json::as_str).map(str::to_string).unwrap_or(state.ui.theme);
        state.ui.font_family = ui.get("font_family").and_then(Json::as_str).filter(|s| !s.is_empty()).map(str::to_string);
        state.ui.placeholder_text =
            ui.get("placeholder_text").and_then(Json::as_str).map(str::to_string).unwrap_or(state.ui.placeholder_text);
        state.ui.show_clock = ui.get("show_clock").and_then(Json::as_bool).unwrap_or(state.ui.show_clock);
        // Bornées, jamais reprises telles quelles -- même raison que
        // l'ancien ui::theme::load : une valeur JSON absurde (infinie,
        // négative, NaN) atteindrait compute_geometry côté appelant.
        state.ui.window_size = ui
            .get("window_size")
            .and_then(Json::as_f64)
            .filter(|n| n.is_finite())
            .map(|n| (n as i32).clamp(5, 100))
            .unwrap_or(state.ui.window_size);
        state.ui.border = ui
            .get("border")
            .and_then(Json::as_f64)
            .filter(|n| n.is_finite())
            .map(|n| (n as i32).clamp(0, 100))
            .unwrap_or(state.ui.border);
    }
    state.auto_restart_enabled = obj.get("auto_restart_enabled").and_then(Json::as_bool).unwrap_or(state.auto_restart_enabled);
    state.auto_kill_enabled = obj.get("auto_kill_enabled").and_then(Json::as_bool).unwrap_or(state.auto_kill_enabled);
    state.copy_history_enabled = obj.get("copy_history_enabled").and_then(Json::as_bool).unwrap_or(state.copy_history_enabled);
    state.hotkey_enabled = obj.get("hotkey_enabled").and_then(Json::as_bool).unwrap_or(state.hotkey_enabled);
    state.hotkey = obj.get("hotkey").and_then(Json::as_str).map(str::to_string).unwrap_or(state.hotkey);
    state
}

fn push_str_value(out: &mut String, s: &str) {
    out.push('"');
    escape_json_content(s, out);
    out.push('"');
}

fn write_json(state: &State) -> String {
    let mut out = String::with_capacity(384);
    out.push_str("{\n  \"ui\": {\n    \"theme\": ");
    push_str_value(&mut out, &state.ui.theme);
    out.push_str(",\n    \"font_family\": ");
    match &state.ui.font_family {
        Some(f) => push_str_value(&mut out, f),
        None => out.push_str("null"),
    }
    out.push_str(",\n    \"placeholder_text\": ");
    push_str_value(&mut out, &state.ui.placeholder_text);
    out.push_str(&format!(
        ",\n    \"show_clock\": {},\n    \"window_size\": {},\n    \"border\": {}\n  }},\n",
        state.ui.show_clock, state.ui.window_size, state.ui.border
    ));
    out.push_str(&format!(
        "  \"auto_restart_enabled\": {},\n  \"auto_kill_enabled\": {},\n  \"copy_history_enabled\": {},\n  \"hotkey_enabled\": {},\n  \"hotkey\": ",
        state.auto_restart_enabled, state.auto_kill_enabled, state.copy_history_enabled, state.hotkey_enabled
    ));
    push_str_value(&mut out, &state.hotkey);
    out.push_str("\n}\n");
    out
}

/// Réécrit state.json en entier -- best-effort à chaque appelant (voir
/// commit_*), jamais fatal si le disque refuse l'écriture : l'affichage a
/// déjà changé en mémoire, un échec de persistance n'a pas à faire échouer
/// l'action visible.
pub fn save(path: &Path, state: &State) -> Result<(), String> {
    fs::write(path, write_json(state)).map_err(|e| e.to_string())
}

/// Charge, applique `mutate`, réécrit -- partagé par les `commit_*` ci-dessous
/// pour qu'un seul champ changé n'écrase jamais les autres avec une valeur
/// par défaut périmée (contrairement à écrire un `State` reconstruit à la
/// main à chaque site d'appel).
fn commit(path: &Path, mutate: impl FnOnce(&mut State)) -> Result<(), String> {
    // `parse` (pas `load`) : `load` sauvegarde déjà son résultat en sortie,
    // écrire une deuxième fois juste après avec la mutation en plus serait
    // une écriture disque redondante pour rien.
    let mut state = parse(path);
    mutate(&mut state);
    save(path, &state)
}

pub fn commit_theme(path: &Path, name: &str) -> Result<(), String> {
    commit(path, |s| s.ui.theme = name.to_string())
}

pub fn commit_window_size(path: &Path, percent: i32) -> Result<(), String> {
    commit(path, |s| s.ui.window_size = percent)
}

pub fn commit_border(path: &Path, border_px: i32) -> Result<(), String> {
    commit(path, |s| s.ui.border = border_px)
}

pub fn commit_auto_restart_enabled(path: &Path, value: bool) -> Result<(), String> {
    commit(path, |s| s.auto_restart_enabled = value)
}

pub fn commit_auto_kill_enabled(path: &Path, value: bool) -> Result<(), String> {
    commit(path, |s| s.auto_kill_enabled = value)
}

pub fn commit_copy_history_enabled(path: &Path, value: bool) -> Result<(), String> {
    commit(path, |s| s.copy_history_enabled = value)
}

pub fn commit_hotkey_enabled(path: &Path, value: bool) -> Result<(), String> {
    commit(path, |s| s.hotkey_enabled = value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_state_test_{}_{}.json", std::process::id(), name));
        let _ = fs::remove_file(&p);
        p
    }

    #[test]
    fn fichier_absent_donne_les_valeurs_par_defaut() {
        let path = temp_path("missing");
        let state = load(&path);
        assert_eq!(state.ui.theme, "arc-dark");
        assert_eq!(state.ui.font_family, None);
        assert_eq!(state.ui.placeholder_text, "Type to search...");
        assert!(state.ui.show_clock);
        assert_eq!(state.ui.window_size, 30);
        assert_eq!(state.ui.border, 1);
        assert!(state.auto_restart_enabled);
        assert!(!state.auto_kill_enabled);
        assert!(state.copy_history_enabled);
        assert!(state.hotkey_enabled);
        assert_eq!(state.hotkey, "Ctrl+Space");
    }

    #[test]
    fn load_sur_un_chemin_absent_cree_le_fichier() {
        let path = temp_path("creates_on_missing");
        assert!(!path.exists());
        load(&path);
        assert!(path.exists());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn round_trip_save_puis_load_preserve_toutes_les_valeurs() {
        let path = temp_path("round_trip");
        let mut state = State::default();
        state.ui.theme = "day".to_string();
        state.ui.font_family = Some("Segoe UI".to_string());
        state.ui.show_clock = false;
        state.ui.window_size = 55;
        state.ui.border = 4;
        state.auto_restart_enabled = false;
        state.auto_kill_enabled = true;
        state.copy_history_enabled = false;
        state.hotkey_enabled = false;
        state.hotkey = "Ctrl+Alt+F".to_string();
        save(&path, &state).unwrap();

        let reloaded = load(&path);
        assert_eq!(reloaded.ui.theme, "day");
        assert_eq!(reloaded.ui.font_family.as_deref(), Some("Segoe UI"));
        assert!(!reloaded.ui.show_clock);
        assert_eq!(reloaded.ui.window_size, 55);
        assert_eq!(reloaded.ui.border, 4);
        assert!(!reloaded.auto_restart_enabled);
        assert!(reloaded.auto_kill_enabled);
        assert!(!reloaded.copy_history_enabled);
        assert!(!reloaded.hotkey_enabled);
        assert_eq!(reloaded.hotkey, "Ctrl+Alt+F");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn font_family_null_reste_none() {
        let path = temp_path("null_font");
        fs::write(&path, r#"{"ui": {"font_family": null}}"#).unwrap();
        assert_eq!(load(&path).ui.font_family, None);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_theme_ne_touche_pas_aux_autres_champs() {
        let path = temp_path("commit_theme");
        let mut state = State::default();
        state.ui.border = 9;
        state.hotkey = "Ctrl+Alt+Space".to_string();
        save(&path, &state).unwrap();

        commit_theme(&path, "night").unwrap();
        let reloaded = load(&path);
        assert_eq!(reloaded.ui.theme, "night");
        assert_eq!(reloaded.ui.border, 9); // inchangé
        assert_eq!(reloaded.hotkey, "Ctrl+Alt+Space"); // inchangé
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_window_size_puis_commit_border_saccumulent() {
        let path = temp_path("commit_accumulate");
        commit_window_size(&path, 80).unwrap();
        commit_border(&path, 6).unwrap();
        let reloaded = load(&path);
        assert_eq!(reloaded.ui.window_size, 80);
        assert_eq!(reloaded.ui.border, 6);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_auto_restart_puis_copy_history_puis_hotkey_saccumulent() {
        let path = temp_path("commit_bools");
        commit_auto_restart_enabled(&path, false).unwrap();
        commit_auto_kill_enabled(&path, true).unwrap();
        commit_copy_history_enabled(&path, false).unwrap();
        commit_hotkey_enabled(&path, false).unwrap();
        let reloaded = load(&path);
        assert!(!reloaded.auto_restart_enabled);
        assert!(reloaded.auto_kill_enabled);
        assert!(!reloaded.copy_history_enabled);
        assert!(!reloaded.hotkey_enabled);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn echappe_un_nom_de_theme_avec_guillemet_ou_antislash() {
        let path = temp_path("escape");
        let tricky = "a\"b\\c";
        commit_theme(&path, tricky).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        let parsed = Json::parse(&after).unwrap();
        assert_eq!(parsed.get("ui").unwrap().get("theme").unwrap().as_str(), Some(tricky));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn window_size_et_border_hors_bornes_sont_clampes_au_chargement() {
        let path = temp_path("clamp");
        fs::write(&path, r#"{"ui": {"window_size": 99999, "border": -50}}"#).unwrap();
        let state = load(&path);
        assert_eq!(state.ui.window_size, 100);
        assert_eq!(state.ui.border, 0);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn objet_ui_absent_garde_les_valeurs_par_defaut_de_ui() {
        let path = temp_path("no_ui");
        fs::write(&path, r#"{"auto_restart_enabled": false}"#).unwrap();
        let state = load(&path);
        assert_eq!(state.ui.theme, "arc-dark");
        assert!(!state.auto_restart_enabled);
        let _ = fs::remove_file(&path);
    }
}
