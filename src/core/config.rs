//! Charge apps.json : la spec du raccourci global + le catalogue d'applis.
//!
//! Contrairement à restart.json/notes.json (voir core::json_list,
//! silencieux sur un fichier absent/corrompu -- fichiers internes que le
//! lanceur est seul à écrire), une erreur ICI est toujours fatale et
//! remonte telle quelle : apps.json est le catalogue même du lanceur,
//! potentiellement édité à la main -- pas de raison de démarrer quand même
//! avec une liste vide plutôt que de signaler clairement le problème.

use std::fmt;
use std::fs;
use std::path::Path;

use crate::json::Json;

use super::models::App;

pub const DEFAULT_HOTKEY: &str = "ctrl+space";

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

pub fn load_config(path: &Path) -> Result<(String, Vec<App>), ConfigError> {
    let text = fs::read_to_string(path).map_err(ConfigError::Io)?;
    let data = Json::parse(&text).map_err(|e| ConfigError::Json(e.to_string()))?;
    let obj = data.as_object().ok_or(ConfigError::NotAnObject)?;

    // Un "hotkey" non-chaîne repart sur la valeur par défaut plutôt que
    // d'être propagé tel quel (ce qui ne se manifesterait que bien plus
    // tard, en plantage sans rapport quand core::hotkey tente de le
    // parser).
    let hotkey = obj.get("hotkey").and_then(Json::as_str).unwrap_or(DEFAULT_HOTKEY).to_string();

    let mut apps = Vec::new();
    if let Some(raw_apps) = obj.get("apps").and_then(Json::as_array) {
        for a in raw_apps {
            let entry = match a.as_object() {
                Some(o) => o,
                None => continue, // l'entrée n'est pas un objet -- ignorée, pas une erreur de config
            };

            // Une clé "path" VRAIMENT absente est une vraie erreur de
            // config (fatale), distincte d'un "path" présent mais vide/mal
            // typé (ignoré en silence juste en dessous).
            let path_val = match entry.get("path") {
                Some(v) => v,
                None => return Err(ConfigError::Json("entrée apps.json sans 'path'".to_string())),
            };
            let path_ok = matches!(path_val, Json::String(s) if !s.trim().is_empty());
            if !path_ok {
                continue;
            }

            match entry.get("name") {
                // "name" vraiment absent -> même traitement fatal qu'un
                // "path" manquant (App::from_json renvoie une Err pour ce
                // cas).
                None => return Err(ConfigError::Json("entrée apps.json sans 'name'".to_string())),
                // Présent mais du mauvais type ou vide -> ignoré comme un
                // path invalide, pas d'appel à from_dict.
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
    Ok((hotkey, apps))
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
    fn charge_hotkey_et_apps() {
        let path = write_temp(
            "ok",
            r#"{"hotkey":"ctrl+alt+f","apps":[{"name":"Notepad","path":"C:\\n.exe"}]}"#,
        );
        let (hotkey, apps) = load_config(&path).unwrap();
        assert_eq!(hotkey, "ctrl+alt+f");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Notepad");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn hotkey_absent_prend_la_valeur_par_defaut() {
        let path = write_temp("default_hotkey", r#"{"apps":[{"name":"A","path":"a.exe"}]}"#);
        let (hotkey, _) = load_config(&path).unwrap();
        assert_eq!(hotkey, DEFAULT_HOTKEY);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn entree_avec_path_vide_est_ignoree_silencieusement() {
        let path = write_temp(
            "blank_path",
            r#"{"apps":[{"name":"A","path":"   "},{"name":"B","path":"b.exe"}]}"#,
        );
        let (_, apps) = load_config(&path).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "B");
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
        let (_, apps) = load_config(&path).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "B");
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
}
