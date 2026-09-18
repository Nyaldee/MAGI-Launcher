//! Classement par correspondance floue utilisé pour trier les résultats de
//! recherche -- réutilisé tel quel par TOUS les pickers de ui::window
//! (catalogue d'applis, Window Switcher, Sticky Notes, Auto-restart,
//! sélecteur de thème), aucun n'a de lien avec core::models::App : ce
//! module ne connaît que des chaînes brutes, jamais un catalogue d'App
//! complet.

/// Tuple de classement `(tier, span)` pour une requête déjà normalisée
/// (minuscule, accents repliés -- voir normalize()), ou `None` si elle
/// n'apparaît nulle part dans le nom, même en fuzzy :
///   (0, 0) -- le nom commence par la requête
///   (1, 0) -- la requête apparaît telle quelle ailleurs dans le nom
///   (2, span) -- correspondance fuzzy seulement ; span = étendue de
///                la PLUS PETITE fenêtre du nom contenant toutes les
///                lettres de la requête dans l'ordre, donc les
///                correspondances les plus "serrées" sont classées
///                avant les plus étalées.
///
/// La plus petite fenêtre n'est PAS obtenue en ancrant chaque lettre sur sa
/// première occurrence : ancrer trop tôt rate une occurrence plus tardive
/// et bien plus resserrée. Exemple, query="cp" sur "Acronis Cyber Protect" :
/// le premier 'c' ("acronis") donne une plage de 13 jusqu'au 'p'
/// ("protect"), alors que le 'c' de "Cyber" donne 6, la vraie plus petite.
///
/// L'algorithme balaie donc CHAQUE occurrence du DERNIER caractère de la
/// requête et, depuis cette fin fixée, cherche en arrière (rposition) la
/// plus proche occurrence de chaque caractère précédent -- ce qui maximise
/// chaque position intermédiaire et minimise donc la plage pour CETTE fin
/// (preuve par échange). Le minimum sur toutes les fins possibles couvre le
/// cas ci-dessus, où une fin tardive bat une fin précoce.
pub fn match_rank(name_lower: &str, query_lower: &str) -> Option<(u8, usize)> {
    if name_lower.starts_with(query_lower) {
        return Some((0, 0));
    }
    if name_lower.contains(query_lower) {
        return Some((1, 0));
    }
    let name: Vec<char> = name_lower.chars().collect();
    let query: Vec<char> = query_lower.chars().collect();
    let last_char = *query.last()?;
    let mut best_span: Option<usize> = None;
    let mut search_from = 0usize;
    while let Some(rel) = name[search_from..].iter().position(|&c| c == last_char) {
        let end = search_from + rel;
        let mut pos = end;
        let mut matched = true;
        for &c in query[..query.len() - 1].iter().rev() {
            match name[..pos].iter().rposition(|&x| x == c) {
                Some(p) => pos = p,
                None => {
                    matched = false;
                    break;
                }
            }
        }
        if matched {
            let span = end - pos;
            if best_span.is_none_or(|b| span < b) {
                best_span = Some(span);
            }
        }
        search_from = end + 1;
    }
    best_span.map(|s| (2, s))
}

/// Comme match_rank(), mais pour une requête à PLUSIEURS mots (séparés par
/// des espaces) : chaque mot doit se retrouver INDÉPENDAMMENT quelque part
/// dans `name_lower`, dans n'importe quel ordre (ET logique) -- `None` si
/// UN SEUL mot ne correspond nulle part. C'est le point d'entrée utilisé
/// par ui::window (voir fuzzy_filter) ; match_rank() reste le bloc de base
/// appliqué mot par mot. Sur une requête sans espace, la boucle ne tourne
/// qu'une fois et le résultat est identique à match_rank() seul.
///
/// Rang combiné : le pire tier parmi tous les mots (un mot retombé en
/// fuzzy pèse plus que les autres, même si un autre a matché en préfixe),
/// puis la somme des spans -- pour que "plus c'est globalement serré, mieux
/// c'est classé" reste vrai à plusieurs mots.
pub fn match_rank_multi(name_lower: &str, query_lower: &str) -> Option<(u8, usize)> {
    let mut worst_tier = 0u8;
    let mut total_span = 0usize;
    for term in query_lower.split_whitespace() {
        let (tier, span) = match_rank(name_lower, term)?;
        worst_tier = worst_tier.max(tier);
        total_span += span;
    }
    Some((worst_tier, total_span))
}

/// Minuscule + repli des accents latins courants vers leur forme ASCII de
/// base -- pas une décomposition NFD/table Unicode complète, seulement ce
/// qu'on rencontre en usage francophone (noms d'applis/fenêtres). À
/// appliquer aux DEUX côtés d'une comparaison (voir core::models::App::new
/// et ui::window) : sans repli symétrique, une requête "credit" ne
/// retrouverait jamais "Crédit Agricole".
pub fn normalize(s: &str) -> String {
    s.to_lowercase().chars().map(strip_accent).collect()
}

fn strip_accent(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correspondance_prefixe_est_tier_0() {
        assert_eq!(match_rank("visual studio code", "vis"), Some((0, 0)));
    }

    #[test]
    fn correspondance_sous_chaine_est_tier_1() {
        assert_eq!(match_rank("command prompt", "prompt"), Some((1, 0)));
    }

    #[test]
    fn sous_sequence_floue_est_tier_2() {
        assert_eq!(match_rank("visual studio code", "vsc").map(|(t, _)| t), Some(2));
    }

    #[test]
    fn aucune_correspondance_est_none() {
        assert_eq!(match_rank("notepad", "zzz"), None);
    }

    #[test]
    fn plus_petite_fenetre_prefere_l_occurrence_tardive_plus_serree() {
        // Un ancrage naïf sur la première occurrence donnerait 13
        // (acronis..protect) ; la vraie plus petite fenêtre fait 6
        // (Cyber..Protect).
        let (tier, span) = match_rank("acronis cyber protect", "cp").unwrap();
        assert_eq!(tier, 2);
        assert_eq!(span, 6);
    }

    #[test]
    fn egalite_departagee_par_la_plus_petite_plage() {
        let tight = match_rank("a_c", "ac").unwrap();
        let loose = match_rank("a__c", "ac").unwrap();
        assert!(tight.1 < loose.1);
    }

    #[test]
    fn normalize_replie_les_accents_courants() {
        assert_eq!(normalize("Crédit Agricole"), "credit agricole");
        assert_eq!(normalize("ÉCOLE"), "ecole");
    }

    #[test]
    fn recherche_insensible_aux_accents() {
        let name = normalize("Crédit Agricole");
        assert!(match_rank_multi(&name, &normalize("credit")).is_some());
    }

    #[test]
    fn match_rank_multi_egale_match_rank_sur_un_seul_mot() {
        assert_eq!(match_rank_multi("visual studio code", "vis"), match_rank("visual studio code", "vis"));
        assert_eq!(match_rank_multi("visual studio code", "zzz"), match_rank("visual studio code", "zzz"));
    }

    #[test]
    fn match_rank_multi_exige_chaque_mot_independamment_de_l_ordre() {
        // "code visual" ne peut pas matcher via match_rank() seul : l'ordre
        // inversé casse la sous-séquence stricte de gauche à droite. Ici
        // chaque mot est cherché indépendamment.
        assert!(match_rank_multi("visual studio code", "code visual").is_some());
        assert!(match_rank("visual studio code", "code visual").is_none());
    }

    #[test]
    fn match_rank_multi_echoue_si_un_seul_mot_ne_correspond_a_rien() {
        assert!(match_rank_multi("visual studio code", "visual zzz").is_none());
    }
}
