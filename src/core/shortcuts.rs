//! Charge le dossier `shortcuts/` (à côté de apps.json/themes.json) --
//! chaque fichier y trouvé devient une entrée lançable en plus de celles
//! d'apps.json, ajoutée APRÈS elles (voir `super::config::load_all`) pour
//! qu'elles restent prioritaires à rang de correspondance égal.
//!
//! Aucun filtre d'extension : tout fichier passe tel quel à `launch()`,
//! qui s'appuie sur `ShellExecuteExW`. Celle-ci résout nativement les
//! `.lnk` et les associations `.bat`/`.cmd`/`.vbs` comme un double-clic
//! dans l'Explorateur, ce qui évite une dépendance COM (IShellLink) pour
//! ce seul dossier.

use std::path::Path;

use super::models::App;

/// Uniquement les fichiers à la racine de `base_dir/shortcuts`, sans
/// récursion. Dossier absent/illisible -> liste vide et silence : c'est une
/// fonctionnalité optionnelle, pas une erreur de config. Tri par nom de
/// fichier insensible à la casse, pour un ordre stable d'un lancement à
/// l'autre.
pub fn load(base_dir: &Path) -> Vec<App> {
    let dir = base_dir.join("shortcuts");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut paths: Vec<std::path::PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    paths.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());

    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let full_path = path.to_string_lossy().into_owned();
            Some(App::new(name, full_path, None, false))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("magi_shortcuts_test_{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn dossier_absent_renvoie_une_liste_vide() {
        let base = temp_dir("missing");
        let _ = std::fs::remove_dir_all(base.join("shortcuts"));
        assert!(load(&base).is_empty());
    }

    #[test]
    fn charge_chaque_fichier_top_level_quelle_que_soit_l_extension() {
        let base = temp_dir("ok");
        let shortcuts = base.join("shortcuts");
        std::fs::create_dir_all(&shortcuts).unwrap();
        std::fs::write(shortcuts.join("Steam.lnk"), b"").unwrap();
        std::fs::write(shortcuts.join("backup.bat"), b"").unwrap();
        std::fs::write(shortcuts.join("noext"), b"").unwrap();

        let apps = load(&base);
        assert_eq!(apps.len(), 3);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Steam"));
        assert!(names.contains(&"backup"));
        assert!(names.contains(&"noext"));
    }

    #[test]
    fn ignore_les_sous_dossiers() {
        let base = temp_dir("subdir");
        let shortcuts = base.join("shortcuts");
        std::fs::create_dir_all(shortcuts.join("Nested")).unwrap();
        std::fs::write(shortcuts.join("Nested").join("inner.lnk"), b"").unwrap();
        std::fs::write(shortcuts.join("top.lnk"), b"").unwrap();

        let apps = load(&base);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "top");
    }

    #[test]
    fn trie_par_nom_insensible_a_la_casse() {
        let base = temp_dir("sort");
        let shortcuts = base.join("shortcuts");
        std::fs::create_dir_all(&shortcuts).unwrap();
        std::fs::write(shortcuts.join("banana.lnk"), b"").unwrap();
        std::fs::write(shortcuts.join("Apple.lnk"), b"").unwrap();
        std::fs::write(shortcuts.join("cherry.lnk"), b"").unwrap();

        let apps = load(&base);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Apple", "banana", "cherry"]);
    }
}
