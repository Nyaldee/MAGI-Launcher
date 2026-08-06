//! Parsing/formatage de durée pour le mode Timer. Même esprit que
//! calculator.rs : logique pure, sans Win32, gardée à part pour rester
//! lisible/testable.

/// Garde-fou sur la durée analysée en secondes -- protège le `SetTimer`
/// du minuteur, avec une limite généreuse (plus de 3000 ans) puisqu'il
/// n'y a de toute façon aucune raison d'accepter plus grand.
const MAX_SECONDS: u64 = 100_000_000_000;

/// Nombre de secondes, ou `None` si `text` n'est pas une durée valide
/// (vide, non numérique, unité inconnue, zéro, ou au-delà de
/// MAX_SECONDS). Pas de suffixe -> minutes par défaut ("5" == "5m"); seule
/// la première lettre du suffixe compte (s/m/h), donc "5m", "5min" et
/// "5minutes" sont équivalents.
pub fn parse_duration(text: &str) -> Option<u64> {
    let t = text.trim();
    let digits_end = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    if digits_end == 0 {
        return None;
    }
    let (digits, rest) = t.split_at(digits_end);
    let rest = rest.trim_start(); // la regex d'origine autorise \s* entre les chiffres et l'unité
    if !rest.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let value: u64 = digits.parse().ok()?;
    let multiplier = match rest.chars().next() {
        None => 60, // pas de suffixe -> minutes
        Some(c) => match c.to_ascii_lowercase() {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            _ => return None,
        },
    };
    let seconds = value.checked_mul(multiplier)?;
    if seconds > 0 && seconds <= MAX_SECONDS {
        Some(seconds)
    } else {
        None
    }
}

/// `mm:ss`, ou `h:mm:ss` au-delà d'une heure.
pub fn format_remaining(seconds: i64) -> String {
    let seconds = seconds.max(0) as u64;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_seul_par_defaut_en_minutes() {
        assert_eq!(parse_duration("5"), Some(300));
    }

    #[test]
    fn unites_seule_premiere_lettre_compte() {
        assert_eq!(parse_duration("90s"), Some(90));
        assert_eq!(parse_duration("5m"), Some(300));
        assert_eq!(parse_duration("1h"), Some(3600));
        assert_eq!(parse_duration("5min"), Some(300));
        assert_eq!(parse_duration("5minutes"), Some(300));
    }

    #[test]
    fn insensible_a_la_casse_et_aux_espaces() {
        assert_eq!(parse_duration("5M"), Some(300));
        assert_eq!(parse_duration(" 5 m "), Some(300));
    }

    #[test]
    fn rejette_entree_invalide() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("0s"), None);
        assert_eq!(parse_duration("5x"), None);
        assert_eq!(parse_duration("-5m"), None);
    }

    #[test]
    fn rejette_duree_absurdement_grande() {
        assert_eq!(parse_duration("999999999999999h"), None);
    }

    #[test]
    fn formate_le_temps_restant() {
        assert_eq!(format_remaining(65), "1:05");
        assert_eq!(format_remaining(3661), "1:01:01");
        assert_eq!(format_remaining(0), "0:00");
        assert_eq!(format_remaining(-5), "0:00");
    }
}
