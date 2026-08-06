//! Persistance des tableaux JSON plats de chaînes : le moteur générique
//! (load_list/save_list) et ses deux emballages de domaine, notes.json
//! (Sticky Notes) et restart.json (liste de surveillance Auto-restart) --
//! les deux seuls fichiers de config avec cette forme (juste le tableau
//! lui-même, pas d'objet englobant). Regroupés dans un seul fichier plutôt
//! qu'un module par emballage : ni l'un ni l'autre n'a de logique propre à
//! ajouter, chacun ne fait que renommer load_list/save_list.
//!
//! Le lanceur est normalement seul à les écrire (jamais édités à la main),
//! mais Reload (voir ui::window::reload_config) les relit quand même
//! depuis le disque -- couvre le cas d'une modification manuelle pendant
//! que le lanceur tourne.

use std::fs;
use std::path::Path;

use crate::json::{write_string_array, Json};

/// Silencieux sur un fichier absent/corrompu (repart d'une liste vide)
/// plutôt que de faire planter le lanceur pour des données non critiques --
/// à opposer à apps.json (core::config), où le même problème est toujours
/// fatal.
pub fn load_list(path: &Path) -> Vec<String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let data = match Json::parse(&text) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    match data.as_array() {
        Some(items) => items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

pub fn save_list(path: &Path, items: &[String]) -> std::io::Result<()> {
    // Une `String` Rust est toujours de l'UTF-8 valide -- pas de
    // nettoyage à faire ici avant d'écrire, contrairement à un texte
    // pouvant contenir un demi-substitut UTF-16 isolé (ex: une source de
    // copier-coller qui a tronqué un emoji en pleine paire).
    fs::write(path, write_string_array(items))
}

pub fn load_notes(path: &Path) -> Vec<String> {
    load_list(path)
}

pub fn save_notes(path: &Path, notes: &[String]) -> std::io::Result<()> {
    save_list(path, notes)
}

/// `path` au sens d'une entrée apps.json (arguments compris) -- voir
/// core::supervisor pour la résolution de la cible au lancement/à la
/// surveillance.
pub fn load_restart_list(path: &Path) -> Vec<String> {
    load_list(path)
}

pub fn save_restart_list(path: &Path, targets: &[String]) -> std::io::Result<()> {
    save_list(path, targets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_json_list_test_{}_{}.json", std::process::id(), name));
        p
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        assert_eq!(load_list(&path), Vec::<String>::new());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let path = temp_path("roundtrip");
        let items = vec!["one".to_string(), "two \"quoted\"".to_string()];
        save_list(&path, &items).unwrap();
        assert_eq!(load_list(&path), items);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn non_string_elements_are_dropped() {
        let path = temp_path("mixed_types");
        fs::write(&path, r#"["a", 5, null, "b"]"#).unwrap();
        assert_eq!(load_list(&path), vec!["a".to_string(), "b".to_string()]);
        let _ = fs::remove_file(&path);
    }
}
