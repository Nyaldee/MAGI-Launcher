//! Lecture de `emoji-test.txt` (fichier officiel Unicode, format UTS #51,
//! placé à côté de l'exe comme apps.json/themes.json -- voir
//! https://www.unicode.org/Public/emoji/latest/emoji-test.txt) pour le mode
//! Emoji : chercher un emoji par son nom anglais officiel et le copier.
//!
//! Aucun JSON maison, aucune table d'emoji embarquée dans le binaire --
//! juste ce fichier texte, remplaçable par une version plus récente
//! d'Unicode sans recompiler (voir Reload). Format d'une ligne de données :
//! `<codepoints hex> ; <statut> # <glyphe> <tag de version> <nom>` -- le
//! glyphe rendu est déjà présent tel quel dans le commentaire, inutile de
//! recomposer les points de code UTF-32 en caractère(s) nous-mêmes.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct EmojiEntry {
    pub name: String,
    pub glyph: String,
}

pub struct EmojiData {
    pub version: String,
    pub entries: Vec<EmojiEntry>,
}

/// `None` uniquement si le fichier est absent/illisible -- un fichier lisible
/// mais sans ligne "# Version:" ou sans entrée exploitable retombe sur une
/// version "?"/une liste vide plutôt que sur `None`, pour ne pas confondre
/// "fichier manquant" et "fichier présent mais inhabituel".
pub fn load(path: &Path) -> Option<EmojiData> {
    let text = fs::read_to_string(path).ok()?;
    let mut version = None;
    let mut entries = Vec::new();
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("# Version:") {
            version = Some(v.trim().to_string());
            continue;
        }
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some(entry) = parse_data_line(line) {
            entries.push(entry);
        }
    }
    Some(EmojiData { version: version.unwrap_or_else(|| "?".to_string()), entries })
}

/// `None` sur une ligne qui n'est pas "fully-qualified" (les autres statuts
/// -- component/minimally-qualified/unqualified -- sont soit des briques
/// internes soit des doublons historiques du même emoji, voir le
/// commentaire d'en-tête du fichier lui-même) ou dont le format ne
/// correspond pas à ce qui est attendu -- une ligne imprévue est ignorée,
/// pas fatale pour le reste du fichier.
fn parse_data_line(line: &str) -> Option<EmojiEntry> {
    let (_codepoints, rest) = line.split_once(';')?;
    let (status, comment) = rest.split_once('#')?;
    if status.trim() != "fully-qualified" {
        return None;
    }
    let comment = comment.trim();
    let (glyph, rest) = comment.split_once(' ')?;
    let (_version_tag, name) = rest.trim_start().split_once(' ')?;
    Some(EmojiEntry { name: name.trim().to_string(), glyph: glyph.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# emoji-test.txt
# Version: 17.0
#
# group: Smileys & Emotion
# subgroup: face-smiling
1F600                                                  ; fully-qualified     # \u{1F600} E1.0 grinning face
263A FE0F                                              ; fully-qualified     # \u{263A}\u{FE0F} E0.6 smiling face
263A                                                   ; unqualified         # \u{263A} E0.6 smiling face
1F468 200D 2764 FE0F 200D 1F468                        ; fully-qualified     # \u{1F468}\u{200D}\u{2764}\u{FE0F}\u{200D}\u{1F468} E2.0 couple with heart: man, man
";

    fn write_temp(name: &str, contents: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("magi_emoji_test_{}_{}.txt", std::process::id(), name));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn charge_la_version_et_ne_garde_que_le_fully_qualified() {
        let path = write_temp("ok", SAMPLE);
        let data = load(&path).unwrap();
        assert_eq!(data.version, "17.0");
        // 3 lignes "fully-qualified" dans l'échantillon -- la 3e ligne
        // ("unqualified", doublon du sourire) est écartée.
        assert_eq!(data.entries.len(), 3);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn preserve_la_ponctuation_dans_un_nom_a_plusieurs_mots() {
        let path = write_temp("punct", SAMPLE);
        let data = load(&path).unwrap();
        let couple = data.entries.iter().find(|e| e.name.contains("couple")).unwrap();
        assert_eq!(couple.name, "couple with heart: man, man");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn extrait_le_bon_glyphe() {
        let path = write_temp("glyph", SAMPLE);
        let data = load(&path).unwrap();
        let grinning = data.entries.iter().find(|e| e.name == "grinning face").unwrap();
        assert_eq!(grinning.glyph, "\u{1F600}");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn fichier_absent_renvoie_none() {
        let path = std::env::temp_dir().join("magi_emoji_definitely_missing.txt");
        let _ = fs::remove_file(&path);
        assert!(load(&path).is_none());
    }

    #[test]
    fn charge_le_vrai_emoji_test_txt_du_projet() {
        // Contre le vrai fichier livré à côté de l'exe (voir Compile.bat/
        // Start.bat), pas un échantillon synthétique -- attrape un vrai
        // souci de format sur le fichier réellement utilisé, pas seulement
        // sur ce que SAMPLE ci-dessus simule.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("emoji-test.txt");
        let data = load(&path).expect("emoji-test.txt devrait être présent à la racine du projet");
        assert_ne!(data.version, "?");
        // Large marge plutôt qu'un compte exact : robuste à une mise à jour
        // du fichier vers une version Unicode ultérieure avec plus d'emoji.
        assert!(data.entries.len() > 3000, "seulement {} entrées", data.entries.len());
        let grinning = data.entries.iter().find(|e| e.name == "grinning face");
        assert_eq!(grinning.map(|e| e.glyph.as_str()), Some("\u{1F600}"));
    }
}
