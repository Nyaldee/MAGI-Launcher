//! Entrée du catalogue de lancement : nom, chemin à ouvrir, dossier de
//! travail optionnel, décodage depuis apps.json -- le classement par
//! correspondance floue lui-même vit dans core::search (partagé avec tous
//! les autres pickers de ui::window, qui n'ont pas d'App).

use crate::core::search::{match_rank, normalize};
use crate::json::Json;

#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub path: String,
    pub cwd: Option<String>,
    pub hidden: bool,
    /// Précalculé une seule fois pour que match_rank() ne remette pas en
    /// minuscules/accents à chaque frappe pour chaque appli du catalogue.
    pub name_lower: String,
}

impl App {
    pub fn new(name: String, path: String, cwd: Option<String>, hidden: bool) -> App {
        let name_lower = normalize(&name);
        App { name, path, cwd, hidden, name_lower }
    }

    /// Construit une App à partir d'une entrée décodée de apps.json : un
    /// `name`/`path` réellement absent est une erreur de config que
    /// l'appelant (config.rs) est censé avoir déjà filtrée (les entrées de
    /// type/vide invalide sont écartées avant d'arriver ici), donc une clé
    /// manquante ici est traitée pareil -- une `Err`, fatale pour
    /// l'appelant.
    pub fn from_json(d: &Json) -> Result<App, String> {
        let name = d.get("name").and_then(Json::as_str).ok_or("champ 'name' manquant")?.to_string();
        let path = d.get("path").and_then(Json::as_str).ok_or("champ 'path' manquant")?.to_string();
        let cwd = d.get("cwd").and_then(Json::as_str).map(str::to_string);
        let hidden = match d.get("hidden") {
            None => false,
            Some(Json::String(s)) => {
                // Piège d'édition manuelle : "hidden": "false" (chaîne
                // JSON) au lieu de "hidden": false (booléen) -- une chaîne
                // non vide est toujours truthy dans une conversion bool
                // naïve, ce qui cacherait l'appli à l'exact inverse de ce
                // que l'utilisateur voulait dire.
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

    /// Tuple de classement `(tier, span)` -- voir core::search::match_rank
    /// pour la sémantique complète.
    // Non utilisée hors des tests : ui::window appelle directement
    // core::search::match_rank_multi() sur `mode_items` (voir son
    // commentaire) pour pouvoir trier apps/fenêtres/notes/cibles/thèmes par
    // le même chemin -- gardée comme méthode d'instance pratique pour les
    // tests et toute utilisation directe d'un `App` isolé.
    #[allow(dead_code)]
    pub fn match_rank(&self, query_lower: &str) -> Option<(u8, usize)> {
        match_rank(&self.name_lower, query_lower)
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
        assert_eq!(a.hidden, false);

        let d2 = Json::parse(r#"{"name":"App","path":"C:\\a.exe","hidden":"yes"}"#).unwrap();
        let a2 = App::from_json(&d2).unwrap();
        assert_eq!(a2.hidden, true);
    }

    #[test]
    fn from_json_path_manquant_est_une_erreur() {
        let d = Json::parse(r#"{"name":"App"}"#).unwrap();
        assert!(App::from_json(&d).is_err());
    }
}
