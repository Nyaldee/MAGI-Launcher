//! Lecteur JSON écrit à la main + un petit writer, sans dépendre de serde.
//! Le côté lecture couvre tout ce dont apps.json/themes.json ont besoin
//! (objets/tableaux imbriqués, chaînes avec échappements, nombres,
//! booléens, null). Le côté écriture n'a besoin de produire que le format
//! tableau plat de chaînes utilisé par notes.json/restart.json (voir
//! core::json_list) -- le thème actif de themes.json est corrigé en place
//! via un remplacement de sous-chaîne ciblé à la place (voir ui::theme),
//! donc un writer complet d'objets/tableaux préservant le formatage édité
//! à la main n'a jamais été nécessaire.
//!
//! Le parseur avance directement sur les OCTETS de la `&str` d'entrée
//! (`pos` est un vrai offset en octets), jamais sur un `Vec<char>` collecté
//! en amont -- même principe que `SliceRead`/`StrRead` dans serde_json
//! (`read.rs`) : toute la grammaire JSON structurelle (espaces,
//! ponctuation, chiffres, littéraux `true`/`false`/`null`) est ASCII, donc
//! un octet suffit à la reconnaître sans jamais tomber au milieu d'un
//! caractère multi-octets. Seul le contenu réel d'une chaîne peut être
//! multi-octets ; il est décodé caractère par caractère via le décodeur
//! UTF-8 de la stdlib (`str::chars`), jamais par du bit-twiddling manuel.
//! Un `Vec<char>` en amont aurait, lui, coûté une allocation O(n) à 4
//! octets par caractère même pour du texte ASCII, ET désynchronisé `pos`
//! (un compte de CARACTÈRES) du message d'erreur ("erreur JSON à l'octet
//! {pos}") dès qu'un caractère accentué précédait l'erreur -- fréquent ici,
//! une appli française.

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

/// Profondeur d'imbrication maximale -- apps.json/themes.json ne descendent
/// jamais à plus de 3-4 niveaux en usage normal ; sans cette limite, un
/// fichier corrompu/malveillant avec des crochets imbriqués à l'infini fait
/// planter tout le processus (dépassement de pile, `parse_value` s'appelant
/// récursivement pour chaque niveau) au lieu de simplement échouer proprement
/// comme n'importe quel autre JSON invalide.
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
    /// Offset en OCTETS dans `text` (pas un compte de caractères) --
    /// s'aligne directement sur le message d'erreur ("à l'octet {pos}") et
    /// permet de trancher `text[a..b]` sans jamais retraverser depuis le
    /// début.
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, message: &str) -> ParseError {
        ParseError { message: message.to_string(), pos: self.pos }
    }

    /// Octet à `pos`, sans avancer -- suffit à reconnaître toute la
    /// grammaire JSON structurelle (espaces, ponctuation, chiffres,
    /// littéraux), entièrement ASCII : jamais un octet de continuation
    /// UTF-8 (>= 0x80). Le contenu réel d'une chaîne, potentiellement
    /// multi-octets, passe par next_char() plus bas, pas par ici.
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

    /// Décode et consomme UN caractère Unicode complet à `pos` via le
    /// décodeur UTF-8 de la stdlib (`str::chars`, jamais de bit-twiddling
    /// manuel) -- utilisé pour le contenu réel des chaînes JSON, qui peut
    /// être multi-octets contrairement au reste de la grammaire.
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
            // Le caractère inattendu peut lui-même être multi-octets (ex:
            // un JSON qui commence directement par du texte non-ASCII) --
            // next_char() plutôt que le seul octet brut, pour un message
            // d'erreur lisible. Sûr : peek_byte() vient de confirmer qu'il
            // reste au moins un octet à `pos`, et `pos` est toujours sur
            // une frontière de caractère valide à ce stade.
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
                            // Paire de substituts (surrogate pair) : un
                            // substitut haut isolé doit être suivi d'un
                            // \uXXXX substitut bas pour former un seul
                            // vrai point de code, mêmes règles UTF-16 que
                            // la norme JSON.
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
        // start..pos ne couvre que des octets ASCII (chiffres/signe/point/
        // exposant) -- toujours une frontière de caractère valide, cette
        // slice ne peut donc jamais paniquer ici.
        self.text[start..self.pos].parse::<f64>().map(Json::Number).map_err(|_| self.err("nombre invalide"))
    }
}

/// Échappe le CONTENU d'une chaîne comme le fait `json.dump(...,
/// ensure_ascii=False)` (sans les guillemets englobants) : seuls '"', '\\'
/// et les caractères de contrôle (<0x20) sont échappés, le texte non-ASCII
/// est laissé en UTF-8 littéral. Publique : réutilisée par ui::theme pour
/// injecter un nom de thème arbitraire entre des guillemets déjà existants
/// (voir commit_theme) sans casser le JSON si ce nom contient lui-même un
/// '"' ou un '\\'.
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
        // "café" fait 4 CARACTÈRES mais 5 OCTETS (é = 2 octets en UTF-8) --
        // avant le passage au curseur en octets, `pos` comptait les
        // caractères (via un Vec<char> collecté en amont), donnant un
        // "octet 7" trompeur pour ce texte au lieu du vrai octet 8.
        let text = "\"café\" x";
        let err = Json::parse(text).unwrap_err();
        assert_eq!(err.pos, 8);
        assert_eq!(&text[err.pos..], "x");
    }

    #[test]
    fn parse_un_caractere_non_ascii_inattendu_sans_planter() {
        // Le tout premier caractère est déjà invalide (accentué,
        // multi-octets) -- exerce next_char() dans la branche générique de
        // parse_value(), pas seulement peek_byte().
        assert!(Json::parse("é").is_err());
    }

    #[test]
    fn ecrit_un_tableau_de_chaines_au_format_python() {
        assert_eq!(write_string_array(&[]), "[]");
        assert_eq!(write_string_array(&["a".to_string()]), "[\n  \"a\"\n]");
        assert_eq!(
            write_string_array(&["a".to_string(), "b".to_string()]),
            "[\n  \"a\",\n  \"b\"\n]"
        );
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
