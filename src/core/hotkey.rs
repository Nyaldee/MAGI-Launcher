//! Enregistrement du raccourci clavier global.
//!
//! Contrainte d'affinité de thread importante : `RegisterHotKey` doit être
//! appelé depuis le MÊME thread qui pompera ensuite `WM_HOTKEY` via
//! `GetMessage` sur le `hwnd` passé ici -- l'appeler depuis un autre
//! thread route silencieusement le message ailleurs. Une seule boucle de
//! messages tourne sur le thread principal, donc `GlobalHotkey::register`
//! doit toujours être appelé depuis ce thread-là.

use crate::win32::user32::{RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
                            VK_ESCAPE, VK_F1, VK_RETURN, VK_SPACE, VK_TAB};
use crate::win32::{last_error, HWND, UINT};

pub const HOTKEY_ID: i32 = 1;

pub struct HotkeySpec {
    pub modifiers: UINT,
    pub vk: UINT,
}

/// Découpe une spec comme "ctrl+space", "ctrl+alt+f", "win+e", "f14".
/// Supporte ctrl/control, alt, shift, win/super, space, enter/return, tab,
/// esc/escape, f1-f24, et les caractères seuls.
pub fn parse_hotkey(spec: &str) -> Result<HotkeySpec, String> {
    let mut modifiers: UINT = 0;
    let mut vk: Option<UINT> = None;

    for part in spec.split('+') {
        let token = part.trim().to_lowercase();
        if token.is_empty() {
            continue;
        }
        match token.as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "super" => modifiers |= MOD_WIN,
            "space" => vk = Some(VK_SPACE as UINT),
            "enter" | "return" => vk = Some(VK_RETURN as UINT),
            "tab" => vk = Some(VK_TAB as UINT),
            "esc" | "escape" => vk = Some(VK_ESCAPE as UINT),
            other => {
                if let Some(rest) = other.strip_prefix('f') {
                    if let Ok(n) = rest.parse::<u32>() {
                        if (1..=24).contains(&n) {
                            vk = Some(VK_F1 as UINT + (n - 1));
                            continue;
                        }
                    }
                }
                let chars: Vec<char> = other.chars().collect();
                if chars.len() == 1 && chars[0].is_ascii_alphanumeric() {
                    vk = Some(chars[0].to_ascii_uppercase() as UINT);
                    continue;
                }
                return Err(format!("jeton de raccourci inconnu '{}'", other));
            }
        }
    }

    let vk = vk.ok_or_else(|| format!("la spec de raccourci '{}' n'a pas de touche non-modificatrice", spec))?;
    Ok(HotkeySpec { modifiers: modifiers | MOD_NOREPEAT, vk })
}

pub struct GlobalHotkey {
    hwnd: HWND,
    registered: bool,
}

impl GlobalHotkey {
    pub fn register(hwnd: HWND, spec: &str) -> Result<GlobalHotkey, String> {
        let parsed = parse_hotkey(spec)?;
        let ok = unsafe { RegisterHotKey(hwnd, HOTKEY_ID, parsed.modifiers, parsed.vk) };
        if ok == 0 {
            return Err(format!("impossible d'enregistrer le raccourci '{}' (erreur {})", spec, last_error()));
        }
        Ok(GlobalHotkey { hwnd, registered: true })
    }

    fn unregister(&mut self) {
        if self.registered {
            unsafe {
                UnregisterHotKey(self.hwnd, HOTKEY_ID);
            }
            self.registered = false;
        }
    }
}

impl Drop for GlobalHotkey {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win32::user32::{MOD_CONTROL, MOD_NOREPEAT, VK_SPACE};

    #[test]
    fn parse_ctrl_space() {
        let hk = parse_hotkey("ctrl+space").unwrap();
        assert_eq!(hk.modifiers, MOD_CONTROL | MOD_NOREPEAT);
        assert_eq!(hk.vk, VK_SPACE as UINT);
    }

    #[test]
    fn parse_touche_fonction() {
        let hk = parse_hotkey("f14").unwrap();
        assert_eq!(hk.vk, 0x70 + 13); // VK_F1 + 13 == VK_F14
    }

    #[test]
    fn parse_caractere_seul() {
        let hk = parse_hotkey("ctrl+alt+f").unwrap();
        assert_eq!(hk.vk, 'F' as UINT);
    }

    #[test]
    fn rejette_modificateurs_seuls() {
        assert!(parse_hotkey("ctrl+alt").is_err());
    }

    #[test]
    fn rejette_jeton_inconnu() {
        assert!(parse_hotkey("ctrl+bogus").is_err());
    }
}
