//! Petit évaluateur arithmétique pour le mode calculatrice de la
//! recherche (taper "2*2" affiche "= 4"). Parseur maison à descente
//! récursive, restreint aux littéraux numériques et aux opérateurs
//! arithmétiques ci-dessous -- aucun chemin vers un `eval` générique, un
//! nom ou un appel de fonction ne peut jamais s'exécuter.
//!
//! Les résultats sont de simples `f64`, pas des entiers à précision
//! arbitraire -- écrire de l'arithmétique bignum à la main pour une
//! calculatrice de confort intégrée serait hors de proportion avec la
//! fonctionnalité, donc les résultats énormes débordent simplement vers
//! +/-inf et sont traités comme invalides, même comportement visible que
//! n'importe quel autre cas de dépassement plus bas.

const MAX_POW_EXPONENT: f64 = 1000.0;

/// Profondeur de récursion maximale du parseur à descente récursive --
/// sans cette limite, une expression collée avec des dizaines de milliers
/// de '(' (ex: "((((...(1") ou de '-' unaires (ex: "-----...-1") fait
/// déborder la pile du thread principal (chaque niveau retraverse toute la
/// chaîne parse_expr/parse_term/parse_factor/parse_power/parse_primary) et
/// plante tout le process -- même risque, même remède que
/// MAX_NESTING_DEPTH dans json.rs. Atteignable directement depuis la barre
/// de recherche : looks_like_expression() accepte ces caractères, aucune
/// EM_LIMITTEXT n'est posée sur le contrôle EDIT (limite par défaut
/// ~32 000 caractères, largement assez pour dépasser cette profondeur).
const MAX_RECURSION_DEPTH: u32 = 64;

/// Filtre rapide avant de tenter un parsing -- une recherche normale
/// ("firefox") ne doit jamais être confondue avec un calcul, et un simple
/// nombre seul ("42") reste une recherche, pas un calcul (il faut au moins
/// un caractère opérateur).
pub fn looks_like_expression(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let allowed = t.chars().all(|c| c.is_ascii_digit() || c.is_whitespace() || ".+-*/%()".contains(c));
    if !allowed {
        return false;
    }
    t.chars().any(|c| "+-*/%".contains(c))
}

/// Retourne le résultat numérique si `expr` est une expression
/// arithmétique valide, `None` sinon (syntaxe invalide, division par
/// zéro, dépassement non fini, résultat complexe issu d'une puissance
/// fractionnaire d'une base négative...).
pub fn evaluate(expr: &str) -> Option<f64> {
    let chars: Vec<char> = expr.chars().collect();
    let mut p = Parser { chars: &chars, pos: 0, depth: 0 };
    let result = p.parse_expr()?;
    p.skip_ws();
    if p.pos != p.chars.len() {
        return None;
    }
    if result.is_finite() {
        Some(result)
    } else {
        None
    }
}

pub fn format_result(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let s = format!("{:.10}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    // expr := term (('+' | '-') term)*
    fn parse_expr(&mut self) -> Option<f64> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('+') => {
                    self.pos += 1;
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.pos += 1;
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Some(value)
    }

    // term := factor (('*' | '/' | '//' | '%') factor)*
    // '**' n'apparaît jamais ici -- parse_power (appelé sous parse_factor)
    // l'a déjà consommé avant que le contrôle ne revienne à cette boucle.
    fn parse_term(&mut self) -> Option<f64> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_ws();
            if self.peek() == Some('/') && self.peek2() == Some('/') {
                self.pos += 2;
                let rhs = self.parse_factor()?;
                if rhs == 0.0 {
                    return None;
                }
                value = (value / rhs).floor();
                continue;
            }
            match self.peek() {
                Some('*') => {
                    self.pos += 1;
                    value *= self.parse_factor()?;
                }
                Some('/') => {
                    self.pos += 1;
                    let rhs = self.parse_factor()?;
                    if rhs == 0.0 {
                        return None;
                    }
                    value /= rhs;
                }
                Some('%') => {
                    self.pos += 1;
                    let rhs = self.parse_factor()?;
                    if rhs == 0.0 {
                        return None;
                    }
                    // Modulo par arrondi vers le bas (le résultat prend le
                    // signe du diviseur), pas un simple reste tronqué.
                    value -= rhs * (value / rhs).floor();
                }
                _ => break,
            }
        }
        Some(value)
    }

    // factor := ('+' | '-') factor | power
    //
    // Seul point d'instrumentation de la profondeur (voir
    // MAX_RECURSION_DEPTH) : chaque niveau de '-'/'+' unaire y repasse
    // directement, et chaque niveau de parenthèses y repasse aussi
    // forcément via parse_term (appelé avant tout opérateur binaire) --
    // pas besoin d'un compteur séparé dans parse_primary.
    fn parse_factor(&mut self) -> Option<f64> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            return None;
        }
        self.skip_ws();
        let result = match self.peek() {
            Some('+') => {
                self.pos += 1;
                self.parse_factor()
            }
            Some('-') => {
                self.pos += 1;
                self.parse_factor().map(|v| -v)
            }
            _ => self.parse_power(),
        };
        self.depth -= 1;
        result
    }

    // power := primary ('**' factor)?  -- associatif à droite, et le côté
    // exposant redescend dans parse_factor (pas parse_power) pour que les
    // signes unaires dans l'exposant fonctionnent aussi (ex: "2**-2").
    fn parse_power(&mut self) -> Option<f64> {
        let base = self.parse_primary()?;
        self.skip_ws();
        if self.peek() == Some('*') && self.peek2() == Some('*') {
            self.pos += 2;
            let exponent = self.parse_factor()?;
            if exponent.abs() > MAX_POW_EXPONENT {
                return None;
            }
            if base < 0.0 && exponent.fract() != 0.0 {
                // ex: (-8)**0.5 -- une paire base/exposant réelle sans
                // résultat réel (le vrai résultat serait un nombre
                // complexe, que format_result ne sait pas afficher) --
                // traité comme une expression invalide plutôt que de
                // laisser un NaN se propager.
                return None;
            }
            return Some(base.powf(exponent));
        }
        Some(base)
    }

    // primary := NOMBRE | '(' expr ')'
    fn parse_primary(&mut self) -> Option<f64> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.pos += 1;
                let v = self.parse_expr()?;
                self.skip_ws();
                if self.peek() != Some(')') {
                    return None;
                }
                self.pos += 1;
                Some(v)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<f64> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek() == Some('.') {
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if self.pos == start {
            return None;
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_expression_exige_un_operateur() {
        assert!(!looks_like_expression("42"));
        assert!(!looks_like_expression("firefox"));
        assert!(looks_like_expression("2+2"));
        assert!(!looks_like_expression(""));
        assert!(!looks_like_expression("  "));
    }

    #[test]
    fn precedence_de_base() {
        assert_eq!(evaluate("3+4*2"), Some(11.0));
        assert_eq!(evaluate("(3+4)*2"), Some(14.0));
        assert_eq!(evaluate("2**3**2"), Some(512.0)); // associatif à droite : 2**(3**2)
        assert_eq!(evaluate("-2**2"), Some(-4.0)); // l'unaire est moins prioritaire que **
    }

    #[test]
    fn division_par_zero_est_invalide() {
        assert_eq!(evaluate("1/0"), None);
        assert_eq!(evaluate("1//0"), None);
        assert_eq!(evaluate("1%0"), None);
    }

    #[test]
    fn expression_incomplete_est_invalide() {
        assert_eq!(evaluate("2+"), None);
        assert_eq!(evaluate("2+*3"), None);
        assert_eq!(evaluate("()"), None);
    }

    #[test]
    fn exposant_enorme_est_rejete() {
        assert_eq!(evaluate("9**999999999"), None);
    }

    #[test]
    fn puissance_fractionnaire_de_base_negative_est_rejetee() {
        assert_eq!(evaluate("(-8)**0.5"), None);
    }

    #[test]
    fn rejette_une_recursion_trop_profonde_sans_planter() {
        let depth = 200_000;
        assert_eq!(evaluate(&("(".repeat(depth) + "1")), None);
        assert_eq!(evaluate(&("-".repeat(depth) + "1")), None);
    }

    #[test]
    fn format_result_suit_le_style_python() {
        assert_eq!(format_result(4.0), "4");
        assert_eq!(format_result(-4.0), "-4");
        assert_eq!(format_result(100.0 / 3.0), "33.3333333333");
        assert_eq!(format_result(0.1 + 0.2), "0.3");
    }
}
