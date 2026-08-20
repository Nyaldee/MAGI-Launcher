//! Lecteur JSON écrit à la main + un petit writer, sans dépendre de serde.
//! La lecture couvre tout ce dont apps.json/themes.json ont besoin
//! (objets/tableaux imbriqués, chaînes avec échappements, nombres, booléens,
//! null). L'écriture ne produit que le tableau plat de chaînes utilisé par
//! notes.json/restart.json (voir core::json_list) : les fichiers édités à la
//! main sont modifiés par remplacement de sous-chaîne ciblé (voir ui::theme)
//! pour préserver leur formatage, jamais resérialisés.
//!
//! Le parseur avance sur les OCTETS de la `&str` d'entrée (`pos` est un
//! offset en octets, pas un compte de caractères). Toute la grammaire JSON
//! structurelle est ASCII, donc un octet suffit à la reconnaître sans jamais
//! tomber au milieu d'un caractère multi-octets ; seul le contenu d'une
//! chaîne peut l'être, et il passe par le décodeur UTF-8 de la stdlib
//! (`str::chars`). Un `Vec<char>` collecté en amont coûterait une allocation
//! O(n) à 4 octets par caractère et désynchroniserait `pos` du message
//! d'erreur ("erreur JSON à l'octet {pos}") sur tout texte accentué.

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(HashMap<String, Json>),
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "erreur JSON à l'octet {} : {}", self.pos, self.message)
    }
}

/// Profondeur d'imbrication maximale : apps.json/themes.json ne dépassent
/// pas 3-4 niveaux en usage normal. Sans cette borne, un fichier corrompu
/// enchaînant les crochets imbriqués fait déborder la pile (`parse_value`
/// récurse par niveau) et abat le processus au lieu d'échouer proprement.
const MAX_NESTING_DEPTH: usize = 64;

impl Json {
    pub fn parse(text: &str) -> Result<Json, ParseError> {
        let mut p = Parser { text, pos: 0, depth: 0 };
        p.skip_ws();
        let value = p.parse_value()?;
        p.skip_ws();
        if p.pos != p.text.len() {
            return Err(p.err("caractères en trop après la valeur JSON"));
        }
        Ok(value)
    }

    pub fn as_object(&self) -> Option<&HashMap<String, Json>> {
        match self {
            Json::Object(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Recherche de champ d'objet, `None` aussi bien quand ce n'est pas un
    /// objet que quand la clé est absente -- les appelants traitent déjà
    /// les deux cas pareil.
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.as_object().and_then(|m| m.get(key))
    }
}

struct Parser<'a> {
    text: &'a str,
    /// Offset en OCTETS dans `text`, pas un compte de caractères : s'aligne
    /// sur le message d'erreur et permet de trancher `text[a..b]` sans
    /// retraverser depuis le début.
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, message: &str) -> ParseError {
        ParseError { message: message.to_string(), pos: self.pos }
    }

    /// Octet à `pos`, sans avancer. Suffit à toute la grammaire structurelle
    /// (ASCII) ; le contenu d'une chaîne passe par `next_char`.
    fn peek_byte(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn bump_byte(&mut self) -> Option<u8> {
        let b = self.peek_byte();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    /// Décode et consomme un caractère Unicode complet à `pos`, via le
    /// décodeur UTF-8 de la stdlib. Réservé au contenu des chaînes JSON,
    /// seule partie de la grammaire qui peut être multi-octets.
    fn next_char(&mut self) -> Option<char> {
        let c = self.text[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek_byte(), Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), ParseError> {
        if self.bump_byte() == Some(b) {
            Ok(())
        } else {
            Err(self.err(&format!("'{}' attendu", b as char)))
        }
    }

    fn expect_literal(&mut self, lit: &str) -> Result<(), ParseError> {
        for expected in lit.bytes() {
            if self.bump_byte() != Some(expected) {
                return Err(self.err(&format!("littéral '{}' attendu", lit)));
            }
        }
        Ok(())
    }

    fn parse_value(&mut self) -> Result<Json, ParseError> {
        self.skip_ws();
        match self.peek_byte() {
            Some(b) if b == b'{' || b == b'[' => {
                self.depth += 1;
                if self.depth > MAX_NESTING_DEPTH {
                    return Err(self.err("imbrication JSON trop profonde"));
                }
                let result = if b == b'{' { self.parse_object() } else { self.parse_array() };
                self.depth -= 1;
                result
            }
            Some(b'"') => self.parse_string().map(Json::String),
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(Json::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(Json::Null)
            }
            Some(b) if b == b'-' || b.is_ascii_digit() => self.parse_number(),
            // Le caractère inattendu peut être multi-octets : next_char()
            // plutôt que l'octet brut, pour un message lisible. L'unwrap est
            // sûr, peek_byte() vient de confirmer qu'il reste un octet et
            // `pos` est sur une frontière de caractère.
            Some(_) => {
                let c = self.next_char().unwrap();
                Err(self.err(&format!("caractère inattendu '{}'", c)))
            }
            None => Err(self.err("fin d'entrée inattendue")),
        }
    }

    fn parse_object(&mut self) -> Result<Json, ParseError> {
        self.expect(b'{')?;
        let mut map = HashMap::new();
        self.skip_ws();
        if self.peek_byte() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(map));
        }
        loop {
            self.skip_ws();
            if self.peek_byte() != Some(b'"') {
                return Err(self.err("clé de type chaîne attendue"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_ws();
            match self.bump_byte() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err(self.err("',' ou '}' attendu dans l'objet")),
            }
        }
        Ok(Json::Object(map))
    }

    fn parse_array(&mut self) -> Result<Json, ParseError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek_byte() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.bump_byte() {
                Some(b',') => continue,
                Some(b']') => break,
                _ => return Err(self.err("',' ou ']' attendu dans le tableau")),
            }
        }
        Ok(Json::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let c = self.next_char().ok_or_else(|| self.err("chaîne non terminée"))?;
            match c {
                '"' => break,
                '\\' => {
                    let esc = self.next_char().ok_or_else(|| self.err("échappement non terminé"))?;
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let hi = self.parse_hex4()?;
                            // Paire de substituts UTF-16 : un substitut haut
                            // doit être suivi d'un \uXXXX substitut bas pour
                            // former un seul point de code.
                            if (0xD800..=0xDBFF).contains(&hi) {
                                if self.next_char() != Some('\\') || self.next_char() != Some('u') {
                                    return Err(self.err("substitut bas attendu"));
                                }
                                let lo = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&lo) {
                                    return Err(self.err("substitut bas invalide"));
                                }
                                let c = 0x10000
                                    + ((hi as u32 - 0xD800) << 10)
                                    + (lo as u32 - 0xDC00);
                                out.push(char::from_u32(c).ok_or_else(|| self.err("paire de substituts invalide"))?);
                            } else {
                                out.push(char::from_u32(hi as u32).ok_or_else(|| self.err("échappement \\u invalide"))?);
                            }
                        }
                        other => return Err(self.err(&format!("échappement invalide '\\{}'", other))),
                    }
                }
                c => out.push(c),
            }
        }
        Ok(out)
    }

    fn parse_hex4(&mut self) -> Result<u16, ParseError> {
        let mut value: u16 = 0;
        for _ in 0..4 {
            let c = self.next_char().ok_or_else(|| self.err("échappement \\u non terminé"))?;
            let digit = c.to_digit(16).ok_or_else(|| self.err("chiffre hexadécimal invalide dans \\u"))?;
            value = value * 16 + digit as u16;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Json, ParseError> {
        let start = self.pos;
        if self.peek_byte() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek_byte(), Some(b) if b.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek_byte() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek_byte(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek_byte(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek_byte(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek_byte(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        // start..pos ne couvre que de l'ASCII (chiffres, signe, point,
        // exposant) : la slice tombe toujours sur une frontière valide.
        self.text[start..self.pos].parse::<f64>().map(Json::Number).map_err(|_| self.err("nombre invalide"))
    }
}

/// Position absolue (en octets) du premier caractère non-blanc suivant la
/// clé `key` et son ':' -- le début de sa valeur JSON, quel que soit son
/// type. Partagée par les éditions en place de apps.json/themes.json
/// (`core::config::commit_bool_setting`, `ui::theme::commit_number`/
/// `commit_theme`) : chacune scanne ensuite l'étendue de la valeur selon son
/// propre type, mais en localiser le début est identique dans les trois cas.
pub fn locate_value_start(text: &str, key: &str) -> Result<usize, String> {
    let quoted_key = format!("\"{key}\"");
    let key_pos = text.find(&quoted_key).ok_or_else(|| format!("clé '{key}' introuvable"))?;
    let after_key = &text[key_pos + quoted_key.len()..];
    let colon_rel = after_key.find(':').ok_or_else(|| format!("':' manquant après '{key}'"))?;
    let after_colon = &after_key[colon_rel + 1..];
    let value_start_rel =
        after_colon.find(|c: char| !c.is_whitespace()).ok_or_else(|| format!("valeur de '{key}' introuvable"))?;
    Ok(key_pos + quoted_key.len() + colon_rel + 1 + value_start_rel)
}

/// Échappe le CONTENU d'une chaîne comme le fait `json.dump(...,
/// ensure_ascii=False)` (sans les guillemets englobants) : seuls '"', '\\'
/// et les caractères de contrôle (<0x20) sont échappés, le texte non-ASCII
/// est laissé en UTF-8 littéral. Publique car ui::theme::commit_theme
/// injecte un nom de thème arbitraire entre des guillemets déjà présents
/// dans le fichier, sans réécrire les délimiteurs.
pub fn escape_json_content(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

fn escape_json_string(s: &str, out: &mut String) {
    out.push('"');
    escape_json_content(s, out);
    out.push('"');
}

/// Écrit un tableau JSON plat de chaînes, indenté sur 2 espaces -- `[]`
/// pour une liste vide (pas de retour à la ligne interne dans ce cas).
pub fn write_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let mut out = String::from("[\n");
    for (i, item) in items.iter().enumerate() {
        out.push_str("  ");
        escape_json_string(item, &mut out);
        if i + 1 < items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_un_objet_plat() {
        let v = Json::parse(r#"{"hotkey": "ctrl+space", "n": 42, "ok": true, "nil": null}"#).unwrap();
        assert_eq!(v.get("hotkey").unwrap().as_str(), Some("ctrl+space"));
        assert_eq!(v.get("n").unwrap().as_f64(), Some(42.0));
        assert_eq!(v.get("ok").unwrap().as_bool(), Some(true));
        assert_eq!(v.get("nil").unwrap(), &Json::Null);
    }

    #[test]
    fn parse_tableaux_et_objets_imbriques() {
        let v = Json::parse(r#"{"apps": [{"name": "A", "path": "C:\\a.exe"}]}"#).unwrap();
        let apps = v.get("apps").unwrap().as_array().unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].get("path").unwrap().as_str(), Some("C:\\a.exe"));
    }

    #[test]
    fn parse_echappements_et_unicode() {
        let v = Json::parse(r#""line1\nline2\t\u00e9\ud83d\ude00""#).unwrap();
        assert_eq!(v.as_str().unwrap(), "line1\nline2\t\u{00e9}\u{1F600}");
    }

    #[test]
    fn parse_nombres() {
        assert_eq!(Json::parse("3.14").unwrap().as_f64(), Some(3.14));
        assert_eq!(Json::parse("-2").unwrap().as_f64(), Some(-2.0));
        assert_eq!(Json::parse("1e3").unwrap().as_f64(), Some(1000.0));
        assert_eq!(Json::parse("0.30").unwrap().as_f64(), Some(0.30));
    }

    #[test]
    fn rejette_json_mal_forme() {
        assert!(Json::parse("{").is_err());
        assert!(Json::parse("[1, 2,]").is_err());
        assert!(Json::parse("not json").is_err());
    }

    #[test]
    fn rejette_une_imbrication_trop_profonde_sans_planter() {
        let depth = 200_000;
        let text = "[".repeat(depth) + &"]".repeat(depth);
        assert!(Json::parse(&text).is_err());
    }

    #[test]
    fn la_position_d_erreur_est_un_offset_en_octets_pas_en_caracteres() {
        // "café" fait 4 caractères mais 5 octets (é = 2 octets en UTF-8) :
        // un `pos` qui compterait les caractères annoncerait l'octet 7.
        let text = "\"café\" x";
        let err = Json::parse(text).unwrap_err();
        assert_eq!(err.pos, 8);
        assert_eq!(&text[err.pos..], "x");
    }

    #[test]
    fn parse_un_caractere_non_ascii_inattendu_sans_planter() {
        // Premier caractère invalide ET multi-octets : exerce next_char()
        // dans la branche générique de parse_value(), pas peek_byte().
        assert!(Json::parse("é").is_err());
    }

    #[test]
    fn ecrit_un_tableau_de_chaines_indente_sur_plusieurs_lignes() {
        assert_eq!(write_string_array(&[]), "[]");
        assert_eq!(write_string_array(&["a".to_string()]), "[\n  \"a\"\n]");
        assert_eq!(
            write_string_array(&["a".to_string(), "b".to_string()]),
            "[\n  \"a\",\n  \"b\"\n]"
        );
    }

    #[test]
    fn locate_value_start_pointe_juste_apres_les_espaces() {
        let text = r#"{"a": true, "hotkey_enabled"  :   false, "n": 5}"#;
        let start = locate_value_start(text, "hotkey_enabled").unwrap();
        assert_eq!(&text[start..start + 5], "false");
    }

    #[test]
    fn locate_value_start_echoue_sur_cle_absente() {
        assert!(locate_value_start(r#"{"a": true}"#, "missing").is_err());
    }

    #[test]
    fn aller_retour_ecriture_puis_lecture_preserve_les_chaines() {
        let items = vec!["hello \"world\"".to_string(), "back\\slash".to_string(), "tab\there".to_string()];
        let written = write_string_array(&items);
        let parsed = Json::parse(&written).unwrap();
        let arr = parsed.as_array().unwrap();
        for (a, b) in arr.iter().zip(items.iter()) {
            assert_eq!(a.as_str().unwrap(), b);
        }
    }
}
