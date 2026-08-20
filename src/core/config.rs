//! Charge apps.json : la spec du raccourci global + le catalogue d'applis.
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

pub const DEFAULT_HOTKEY: &str = "ctrl+space";

/// Contenu utile de apps.json : le catalogue lui-même, plus les quelques
/// bascules du menu tray (hotkey/auto-restart/copy-history) qui doivent
/// survivre à un redémarrage -- lues ici avec leur valeur par défaut
/// respective, écrites individuellement via `commit_bool_setting` à chaque
/// bascule (voir main.rs/ui::window).
pub struct Config {
    pub hotkey: String,
    pub apps: Vec<App>,
    pub hotkey_enabled: bool,
    pub auto_restart_enabled: bool,
    /// `false` par défaut : fonctionnalité opt-in (voir
    /// core::clipboard_history), contrairement aux deux bascules ci-dessus.
    pub copy_history_enabled: bool,
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

    // Un "hotkey" non-chaîne repart sur la valeur par défaut : le propager
    // tel quel ne se manifesterait que bien plus tard, à son parsing par
    // core::hotkey.
    let hotkey = obj.get("hotkey").and_then(Json::as_str).unwrap_or(DEFAULT_HOTKEY).to_string();
    let hotkey_enabled = obj.get("hotkey_enabled").and_then(Json::as_bool).unwrap_or(true);
    let auto_restart_enabled = obj.get("auto_restart_enabled").and_then(Json::as_bool).unwrap_or(true);
    let copy_history_enabled = obj.get("copy_history_enabled").and_then(Json::as_bool).unwrap_or(false);

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
    Ok(Config { hotkey, apps, hotkey_enabled, auto_restart_enabled, copy_history_enabled })
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

/// Remplace (ou insère si absente) la valeur BOOLÉENNE `key` à la racine de
/// apps.json par `value` -- même remplacement de sous-chaîne ciblé que
/// `ui::theme::commit_theme`/`commit_number` (voir `json::locate_value_start`,
/// partagé par les trois), pour préserver le reste du fichier intact.
/// Contrairement à ces deux-là, la clé peut être totalement absente d'un
/// apps.json antérieur à ces réglages : elle est alors insérée juste après
/// l'accolade ouvrante.
pub fn commit_bool_setting(path: &Path, key: &str, value: bool) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let quoted_key = format!("\"{key}\"");

    let new_text = if text.contains(&quoted_key) {
        // Clé présente mais malformée (pas de ':', pas de valeur) :
        // `locate_value_start` échoue durement, distinct du cas "absente"
        // ci-dessous qui insère.
        let value_start = crate::json::locate_value_start(&text, key)?;
        // La longueur de la valeur est vérifiée, pas supposée : sur une clé
        // éditée à la main avec autre chose qu'un booléen (ex: un nombre),
        // couper à une longueur fixe sortirait des limites ou grignoterait
        // le mauvais nombre d'octets, corrompant le fichier.
        let rest = &text[value_start..];
        let value_len = if rest.starts_with("true") {
            4
        } else if rest.starts_with("false") {
            5
        } else {
            return Err(format!("valeur de '{key}' n'est ni 'true' ni 'false'"));
        };
        let value_end = value_start + value_len;

        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..value_start]);
        out.push_str(if value { "true" } else { "false" });
        out.push_str(&text[value_end..]);
        out
    } else {
        // Insertion juste après le '{' ouvrant, avec une virgule si la
        // racine n'est pas vide.
        let brace_pos = text.find('{').ok_or("'{' introuvable dans apps.json")?;
        let insert_pos = brace_pos + 1;
        let rest_is_empty = text[insert_pos..].trim_start().starts_with('}');
        let mut out = String::with_capacity(text.len() + 32);
        out.push_str(&text[..insert_pos]);
        out.push_str(&format!("\n  \"{key}\": {value}"));
        if !rest_is_empty {
            out.push(',');
        }
        out.push_str(&text[insert_pos..]);
        out
    };

    fs::write(path, new_text).map_err(|e| e.to_string())
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
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkey, "ctrl+alt+f");
        assert_eq!(cfg.apps.len(), 1);
        assert_eq!(cfg.apps[0].name, "Notepad");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn hotkey_absent_prend_la_valeur_par_defaut() {
        let path = write_temp("default_hotkey", r#"{"apps":[{"name":"A","path":"a.exe"}]}"#);
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.hotkey, DEFAULT_HOTKEY);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn bascules_absentes_prennent_leur_valeur_par_defaut() {
        let path = write_temp("default_toggles", r#"{"apps":[{"name":"A","path":"a.exe"}]}"#);
        let cfg = load_config(&path).unwrap();
        assert!(cfg.hotkey_enabled);
        assert!(cfg.auto_restart_enabled);
        assert!(!cfg.copy_history_enabled);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn bascules_explicites_sont_lues() {
        let path = write_temp(
            "explicit_toggles",
            r#"{"apps":[{"name":"A","path":"a.exe"}],"hotkey_enabled":false,"auto_restart_enabled":false,"copy_history_enabled":true}"#,
        );
        let cfg = load_config(&path).unwrap();
        assert!(!cfg.hotkey_enabled);
        assert!(!cfg.auto_restart_enabled);
        assert!(cfg.copy_history_enabled);
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

    #[test]
    fn commit_bool_setting_remplace_une_cle_existante() {
        let path = write_temp("commit_replace", r#"{"hotkey_enabled": true, "apps": []}"#);
        commit_bool_setting(&path, "hotkey_enabled", false).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("\"hotkey_enabled\": false"));
        let parsed = crate::json::Json::parse(&after).unwrap();
        assert_eq!(parsed.get("hotkey_enabled").unwrap().as_bool(), Some(false));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_bool_setting_insere_une_cle_absente() {
        let path = write_temp("commit_insert", r#"{"apps": []}"#);
        commit_bool_setting(&path, "copy_history_enabled", true).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        let parsed = crate::json::Json::parse(&after).unwrap();
        assert_eq!(parsed.get("copy_history_enabled").unwrap().as_bool(), Some(true));
        assert_eq!(parsed.get("apps").unwrap().as_array().unwrap().len(), 0);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_bool_setting_echoue_proprement_sur_une_valeur_ni_vraie_ni_fausse() {
        // Édition manuelle malformée (nombre à la place d'un booléen) :
        // doit renvoyer Err, jamais paniquer ni réécrire le fichier.
        let path = write_temp("commit_bad_value", r#"{"hotkey_enabled": 1, "apps": []}"#);
        let before = fs::read_to_string(&path).unwrap();
        assert!(commit_bool_setting(&path, "hotkey_enabled", false).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before, "le fichier ne doit pas être touché en cas d'échec");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn commit_bool_setting_insere_dans_un_objet_vide() {
        let path = write_temp("commit_insert_empty", r#"{}"#);
        commit_bool_setting(&path, "hotkey_enabled", false).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        let parsed = crate::json::Json::parse(&after).unwrap();
        assert_eq!(parsed.get("hotkey_enabled").unwrap().as_bool(), Some(false));
        let _ = fs::remove_file(&path);
    }
}
