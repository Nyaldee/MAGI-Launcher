//! Entrée du catalogue de lancement : nom, chemin à ouvrir, dossier de
//! travail optionnel, décodage depuis apps.json -- le classement par
//! correspondance floue lui-même vit dans core::search (partagé avec tous
//! les autres pickers de ui::window, qui n'ont pas d'App).

use crate::json::Json;

#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub path: String,
    pub cwd: Option<String>,
    pub hidden: bool,
}

impl App {
    pub fn new(name: String, path: String, cwd: Option<String>, hidden: bool) -> App {
        App { name, path, cwd, hidden }
    }

    /// Construit une App depuis une entrée décodée de apps.json. `name` et
    /// `path` sont obligatoires : leur absence est une erreur de config,
    /// fatale pour l'appelant (config.rs), au même titre que les entrées de
    /// type invalide qu'il écarte déjà en amont.
    pub fn from_json(d: &Json) -> Result<App, String> {
        let name = d.get("name").and_then(Json::as_str).ok_or("champ 'name' manquant")?.to_string();
        let path = d.get("path").and_then(Json::as_str).ok_or("champ 'path' manquant")?.to_string();
        let cwd = d.get("cwd").and_then(Json::as_str).map(str::to_string);
        let hidden = match d.get("hidden") {
            None => false,
            Some(Json::String(s)) => {
                // Piège d'édition manuelle : "hidden": "false" (chaîne)
                // au lieu de false (booléen). Une conversion bool naïve
                // rend toute chaîne non vide truthy et cacherait l'appli à
                // l'inverse de l'intention.
                let t = s.trim().to_lowercase();
                !(t.is_empty() || t == "false" || t == "0")
            }
            Some(Json::Bool(b)) => *b,
            Some(Json::Number(n)) => *n != 0.0,
            Some(Json::Array(a)) => !a.is_empty(),
            Some(Json::Object(m)) => !m.is_empty(),
            Some(Json::Null) => false,
        };
        Ok(App::new(name, path, cwd, hidden))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_json_lit_les_champs_et_le_piege_hidden_chaine() {
        let d = Json::parse(r#"{"name":"App","path":"C:\\a.exe","hidden":"false"}"#).unwrap();
        let a = App::from_json(&d).unwrap();
        assert_eq!(a.name, "App");
        assert!(!a.hidden);

        let d2 = Json::parse(r#"{"name":"App","path":"C:\\a.exe","hidden":"yes"}"#).unwrap();
        let a2 = App::from_json(&d2).unwrap();
        assert!(a2.hidden);
    }

    #[test]
    fn from_json_path_manquant_est_une_erreur() {
        let d = Json::parse(r#"{"name":"App"}"#).unwrap();
        assert!(App::from_json(&d).is_err());
    }
}
