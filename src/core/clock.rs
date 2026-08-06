//! Formate une heure selon le format court réglé par l'utilisateur Windows
//! (Paramètres > Heure et langue > Formats -- le même que la barre des
//! tâches) plutôt qu'un format 24h codé en dur.

use crate::win32::kernel32::{GetLocalTime, GetTimeFormatEx, SYSTEMTIME, TIME_NOSECONDS};
use crate::win32::from_wstring;

pub struct SimpleTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
}

/// `st` formaté selon le format court par défaut de la locale utilisateur
/// courante (passer None pour le nom de locale ET le format demande
/// exactement ça à GetTimeFormatEx, plutôt qu'un patron qu'on imposerait).
pub fn format_time(st: &SimpleTime) -> String {
    let system_time = SYSTEMTIME {
        wYear: st.year,
        wMonth: st.month,
        wDayOfWeek: 0,
        wDay: st.day,
        wHour: st.hour,
        wMinute: st.minute,
        wSecond: st.second,
        wMilliseconds: 0,
    };
    let mut buf = [0u16; 64];
    unsafe {
        GetTimeFormatEx(
            std::ptr::null(),
            TIME_NOSECONDS,
            &system_time,
            std::ptr::null(),
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
    }
    from_wstring(&buf)
}

/// Heure locale actuelle, formatée -- utilisé par l'horloge de la barre de
/// recherche (voir ui::window), rafraîchi à chaque WM_TIMER d'une seconde.
pub fn format_now() -> String {
    let mut st = SYSTEMTIME::default();
    unsafe {
        GetLocalTime(&mut st);
    }
    format_time(&SimpleTime {
        year: st.wYear,
        month: st.wMonth,
        day: st.wDay,
        hour: st.wHour,
        minute: st.wMinute,
        second: st.wSecond,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formate_une_heure_connue_sans_planter() {
        let st = SimpleTime { year: 2026, month: 1, day: 1, hour: 13, minute: 5, second: 0 };
        let s = format_time(&st);
        // La chaîne exacte dépend de la locale/du format d'heure court de
        // la machine (c'est tout l'intérêt, voir le commentaire du module)
        // -- on vérifie juste qu'on récupère quelque chose de plausible,
        // pas vide ni du charabia.
        assert!(!s.is_empty());
        assert!(s.len() < 20);
    }
}
