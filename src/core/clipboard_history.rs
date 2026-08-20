//! Historique presse-papier RAM-only (texte uniquement), opt-in via le
//! tray (voir ui::window::toggle_copy_history) -- capturé via
//! AddClipboardFormatListener (voir ui::window::wndproc), jamais écrit sur
//! disque : plus aucune trace récupérable une fois le process terminé,
//! quel que soit l'état du disque.
//!
//! Sécurité : `VirtualLock` (page non-swappable, best-effort) + effacement
//! volatile à zéro avant libération, SANS chiffrement. Une clé stockée dans
//! le process ne protège de rien face à un attaquant qui lit déjà sa
//! mémoire ; et un buffer chiffré en RAM correspond au pattern d'un
//! infostealer cherchant à échapper aux scanners mémoire, ce qui exposerait
//! le lanceur aux heuristiques antivirus.

use std::collections::VecDeque;

use crate::win32::kernel32::{VirtualLock, VirtualUnlock};

/// Capacité totale de l'historique, en octets UTF-8 cumulés : ~1 million de
/// caractères ASCII, largement au-dessus d'un usage normal de presse-papier
/// et négligeable pour la mémoire du lanceur.
const MAX_BYTES: usize = 1_000_000;

/// Chaîne dont le buffer est épinglé en RAM (`VirtualLock`, best-effort :
/// un échec dégrade vers de la RAM normale sans désactiver la
/// fonctionnalité) et effacé à zéro à sa destruction -- écriture volatile,
/// que l'optimiseur ne peut pas éliminer comme un `memset` mort juste avant
/// une libération.
struct LockedString {
    text: String,
    locked: bool,
}

impl LockedString {
    fn new(text: String) -> LockedString {
        let locked =
            if text.is_empty() { false } else { unsafe { VirtualLock(text.as_ptr() as *const _, text.len()) != 0 } };
        LockedString { text, locked }
    }
}

impl Drop for LockedString {
    fn drop(&mut self) {
        unsafe {
            for b in self.text.as_bytes_mut().iter_mut() {
                std::ptr::write_volatile(b, 0);
            }
            if self.locked {
                VirtualUnlock(self.text.as_ptr() as *const _, self.text.len());
            }
        }
    }
}

/// Le plus récent en index 0 -- même convention que Sticky Notes.
pub struct ClipboardHistory {
    entries: VecDeque<LockedString>,
    total_len: usize,
}

impl ClipboardHistory {
    pub fn new() -> ClipboardHistory {
        ClipboardHistory { entries: VecDeque::new(), total_len: 0 }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Texte de l'entrée `idx` (0 = la plus récente), pour l'affichage et
    /// la re-copie. Jamais une `String` possédée : rester en `&str` évite
    /// une copie ni verrouillée ni effacée hors de ce module.
    pub fn get(&self, idx: usize) -> Option<&str> {
        self.entries.get(idx).map(|e| e.text.as_str())
    }

    /// Ajoute `text` en tête. Ignore un texte vide/blanc ou identique au
    /// plus récent (Ctrl+C répété sur la même sélection), et rejette une
    /// entrée dépassant à elle seule MAX_BYTES. Sinon évince depuis la fin
    /// (le plus ancien) jusqu'à faire de la place : chaque éviction
    /// déclenche le Drop de LockedString, qui efface son texte avant de
    /// rendre la mémoire.
    pub fn push(&mut self, text: String) {
        if text.trim().is_empty() || text.len() > MAX_BYTES {
            return;
        }
        if self.entries.front().map(|e| e.text.as_str()) == Some(text.as_str()) {
            return;
        }
        while self.total_len + text.len() > MAX_BYTES {
            match self.entries.pop_back() {
                Some(evicted) => self.total_len -= evicted.text.len(),
                None => break,
            }
        }
        self.total_len += text.len();
        self.entries.push_front(LockedString::new(text));
    }

    /// Retire l'entrée `idx` (voir Suppr dans Mode::CopyHistory).
    pub fn remove(&mut self, idx: usize) {
        if idx < self.entries.len() {
            self.total_len -= self.entries.remove(idx).map(|e| e.text.len()).unwrap_or(0);
        }
    }

    /// Vide tout l'historique (Maj+Suppr) -- chaque LockedString retirée
    /// s'efface via son Drop.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_len = 0;
    }
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        ClipboardHistory::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_puis_get_dans_l_ordre_du_plus_recent() {
        let mut h = ClipboardHistory::new();
        h.push("premier".to_string());
        h.push("second".to_string());
        assert_eq!(h.len(), 2);
        assert_eq!(h.get(0), Some("second"));
        assert_eq!(h.get(1), Some("premier"));
    }

    #[test]
    fn ignore_le_texte_vide_ou_blanc() {
        let mut h = ClipboardHistory::new();
        h.push("".to_string());
        h.push("   ".to_string());
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn ignore_un_doublon_immediat_du_plus_recent() {
        let mut h = ClipboardHistory::new();
        h.push("x".to_string());
        h.push("x".to_string());
        assert_eq!(h.len(), 1);
        // Seul le doublon immédiat est filtré : le même texte recopié plus
        // tard reste accepté.
        h.push("y".to_string());
        h.push("x".to_string());
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn rejette_une_entree_qui_depasse_a_elle_seule_la_capacite() {
        let mut h = ClipboardHistory::new();
        h.push("a".repeat(MAX_BYTES + 1));
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn evince_les_plus_anciens_pour_faire_de_la_place() {
        let mut h = ClipboardHistory::new();
        h.push("a".repeat(MAX_BYTES / 2));
        h.push("b".repeat(MAX_BYTES / 2));
        // Les deux tiennent tout juste ensemble.
        assert_eq!(h.len(), 2);
        // Une troisième entrée aussi grosse force l'éviction de la plus
        // ancienne ("a") pour rester sous MAX_BYTES.
        h.push("c".repeat(MAX_BYTES / 2));
        assert_eq!(h.len(), 2);
        assert_eq!(h.get(0), Some(&*"c".repeat(MAX_BYTES / 2)));
        assert_eq!(h.get(1), Some(&*"b".repeat(MAX_BYTES / 2)));
    }

    #[test]
    fn remove_puis_clear() {
        let mut h = ClipboardHistory::new();
        h.push("a".to_string());
        h.push("b".to_string());
        h.push("c".to_string());
        h.remove(1); // index 1 de (c, b, a) = "b"
        assert_eq!(h.len(), 2);
        assert_eq!(h.get(0), Some("c"));
        assert_eq!(h.get(1), Some("a"));
        h.clear();
        assert_eq!(h.len(), 0);
    }
}
