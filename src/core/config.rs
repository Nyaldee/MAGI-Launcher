//! Charge apps.json : le catalogue d'applis. Le raccourci global et les
//! bascules du menu tray vivent dans state.json (voir core::state) --
//! apps.json ne porte plus que ce que son nom dit.
//!
//! Contrairement à restart.json/notes.json (voir core::json_list,
//! silencieux sur un fichier absent/corrompu -- fichiers internes que le
//! lanceur est seul à écrire), une erreur ICI est fatale et remonte telle
//! quelle : apps.json est le catalogue même du lanceur, potentiellement
//! édité à la main, et démarrer avec une liste vide masquerait le problème
//! au lieu de le signaler.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::json::Json;

use super::models::App;

/// Contenu utile de apps.json : uniquement le catalogue.
pub struct Config {
    pub apps: Vec<App>,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Json(String),
    /// apps.json est du JSON valide mais sa racine n'est pas un objet (un
    /// tableau, un simple nombre...).
    NotAnObject,
    /// JSON valide, forme d'objet valide, mais aucune appli utilisable
    /// dedans -- un lanceur sans une seule appli n'a rien à proposer, même
    /// traitement fatal qu'un fichier manquant/mal formé.
    NoApps,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "{}", e),
            ConfigError::Json(msg) => write!(f, "{}", msg),
            ConfigError::NotAnObject => write!(f, "la racine de apps.json n'est pas un objet"),
            ConfigError::NoApps => write!(f, "apps.json ne contient aucune appli"),
        }
    }
}

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let text = fs::read_to_string(path).map_err(ConfigError::Io)?;
    let data = Json::parse(&text).map_err(|e| ConfigError::Json(e.to_string()))?;
    let obj = data.as_object().ok_or(ConfigError::NotAnObject)?;

    let mut apps = Vec::new();
    if let Some(raw_apps) = obj.get("apps").and_then(Json::as_array) {
        for a in raw_apps {
            let entry = match a.as_object() {
                Some(o) => o,
                None => continue, // l'entrée n'est pas un objet -- ignorée, pas une erreur de config
            };

            // Clé "path" absente = erreur de config fatale, distincte d'un
            // "path" présent mais vide/mal typé (ignoré en silence juste en
            // dessous).
            let path_val = match entry.get("path") {
                Some(v) => v,
                None => return Err(ConfigError::Json("entrée apps.json sans 'path'".to_string())),
            };
            let path_ok = matches!(path_val, Json::String(s) if !s.trim().is_empty());
            if !path_ok {
                continue;
            }

            match entry.get("name") {
                // Absent -> fatal, comme un "path" manquant.
                None => return Err(ConfigError::Json("entrée apps.json sans 'name'".to_string())),
                // Présent mais vide ou mal typé -> ignoré, comme un path
                // invalide.
                Some(Json::String(s)) if !s.trim().is_empty() => {
                    apps.push(App::from_json(a).map_err(ConfigError::Json)?);
                }
                Some(_) => continue,
            }
        }
    }

    if apps.is_empty() {
        return Err(ConfigError::NoApps);
    }
    Ok(Config { apps })
}

/// `load_config` (apps.json) + le dossier `shortcuts/` du même répertoire
/// (voir core::shortcuts) -- les raccourcis arrivent APRÈS ceux d'apps.json
/// dans `apps`, donc plus bas dans les résultats à rang de correspondance
/// égal. Point d'entrée unique (démarrage et rechargement) pour que les
/// deux chemins ne puissent pas diverger sur l'ajout des raccourcis.
pub fn load_all(base_dir: &Path) -> Result<Config, ConfigError> {
    let mut config = load_config(&base_dir.join("apps.json"))?;
    config.apps.extend(super::shortcuts::load(base_dir));
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(name: &str, contents: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("magi_config_test_{}_{}.json", std::process::id(), name));
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn charge_les_apps() {
        let path = write_temp("ok", r#"{"apps":[{"name":"Notepad","path":"C:\\n.exe"}]}"#);
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.apps.len(), 1);
        assert_eq!(cfg.apps[0].name, "Notepad");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn entree_avec_path_vide_est_ignoree_silencieusement() {
        let path = write_temp(
            "blank_path",
            r#"{"apps":[{"name":"A","path":"   "},{"name":"B","path":"b.exe"}]}"#,
        );
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.apps.len(), 1);
        assert_eq!(cfg.apps[0].name, "B");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn cle_path_manquante_est_fatale() {
        let path = write_temp("missing_path", r#"{"apps":[{"name":"A"}]}"#);
        assert!(load_config(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn cle_name_manquante_est_fatale() {
        let path = write_temp("missing_name", r#"{"apps":[{"path":"a.exe"}]}"#);
        assert!(load_config(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn type_name_invalide_est_ignore_silencieusement() {
        let path = write_temp(
            "bad_name_type",
            r#"{"apps":[{"name":5,"path":"a.exe"},{"name":"B","path":"b.exe"}]}"#,
        );
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.apps.len(), 1);
        assert_eq!(cfg.apps[0].name, "B");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn catalogue_vide_est_fatal() {
        let path = write_temp("empty", r#"{"apps":[]}"#);
        assert!(matches!(load_config(&path), Err(ConfigError::NoApps)));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn racine_non_objet_est_fatale() {
        let path = write_temp("array_root", r#"[1,2,3]"#);
        assert!(matches!(load_config(&path), Err(ConfigError::NotAnObject)));
        let _ = fs::remove_file(&path);
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_config_dir_test_{}_{}", std::process::id(), tag));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn load_all_ajoute_les_raccourcis_apres_les_apps() {
        let base = temp_dir("load_all");
        fs::write(base.join("apps.json"), r#"{"apps":[{"name":"A","path":"a.exe"}]}"#).unwrap();
        let shortcuts = base.join("shortcuts");
        fs::create_dir_all(&shortcuts).unwrap();
        fs::write(shortcuts.join("Z.lnk"), b"").unwrap();

        let cfg = load_all(&base).unwrap();
        assert_eq!(cfg.apps.len(), 2);
        assert_eq!(cfg.apps[0].name, "A"); // apps.json d'abord
        assert_eq!(cfg.apps[1].name, "Z"); // le raccourci après
    }
}
