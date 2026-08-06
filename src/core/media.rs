//! Simule les touches média/volume pour les entrées du lanceur comme
//! `magi:media-play-pause`. Une première implémentation utilisait
//! SendInput avec VK_MEDIA_*/scancode matériel : Windows acceptait bien
//! l'injection
//! (confirmé : SendInput renvoyait 2/2 évènements), mais rien ne se
//! passait car l'évènement clavier simulé ne concerne QUE la fenêtre qui a
//! le focus au moment de l'appel (la nôtre) -- une vraie touche média est
//! en réalité routée par le Shell vers la session média active via
//! WM_APPCOMMAND envoyé à la fenêtre de la barre des tâches
//! ("Shell_TrayWnd"), pas via un évènement clavier qui resterait local à
//! l'appli qui a le focus. Envoyer ce même WM_APPCOMMAND directement à
//! Shell_TrayWnd reproduit ce routage global, plutôt que de cibler notre
//! propre fenêtre ou celle au premier plan.

use crate::win32::user32::{FindWindowW, SendMessageW, WM_APPCOMMAND};
use crate::win32::to_wstring;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKey {
    PlayPause,
    Next,
    Previous,
    Stop,
    VolumeMute,
    VolumeDown,
    VolumeUp,
}

/// APPCOMMAND_* (winuser.h) -- pas exposées par windows-sys (famille
/// d'en-tête séparée, hors du périmètre généré), reprises ici en dur :
/// constantes numériques stables documentées, jamais censées changer.
const APPCOMMAND_MEDIA_NEXTTRACK: u16 = 11;
const APPCOMMAND_MEDIA_PREVIOUSTRACK: u16 = 12;
const APPCOMMAND_MEDIA_STOP: u16 = 13;
const APPCOMMAND_MEDIA_PLAY_PAUSE: u16 = 14;
const APPCOMMAND_VOLUME_MUTE: u16 = 8;
const APPCOMMAND_VOLUME_DOWN: u16 = 9;
const APPCOMMAND_VOLUME_UP: u16 = 10;
const FAPPCOMMAND_KEY: u16 = 0;

impl MediaKey {
    fn appcommand(self) -> u16 {
        match self {
            MediaKey::PlayPause => APPCOMMAND_MEDIA_PLAY_PAUSE,
            MediaKey::Next => APPCOMMAND_MEDIA_NEXTTRACK,
            MediaKey::Previous => APPCOMMAND_MEDIA_PREVIOUSTRACK,
            MediaKey::Stop => APPCOMMAND_MEDIA_STOP,
            MediaKey::VolumeMute => APPCOMMAND_VOLUME_MUTE,
            MediaKey::VolumeDown => APPCOMMAND_VOLUME_DOWN,
            MediaKey::VolumeUp => APPCOMMAND_VOLUME_UP,
        }
    }
}

/// Envoie la commande média donnée à Shell_TrayWnd -- `false` si cette
/// fenêtre (la barre des tâches elle-même) est introuvable, ce qui ne
/// devrait jamais arriver sur un Explorateur Windows en fonctionnement
/// normal, mais reste vérifié plutôt que de risquer un SendMessageW sur un
/// HWND nul.
pub fn send_media_key(key: MediaKey) -> bool {
    unsafe {
        let class_name = to_wstring("Shell_TrayWnd");
        let tray_hwnd = FindWindowW(class_name.as_ptr(), std::ptr::null());
        if tray_hwnd.is_null() {
            return false;
        }
        // HIWORD(lParam) = cmd | device, SANS décalage de cmd -- Windows
        // extrait la commande via `HIWORD & ~0xF000` (les 4 bits hauts sont
        // réservés au type de périphérique, FAPPCOMMAND_KEY=0 ici) : un
        // décalage supplémentaire de la commande (bug précédent, `cmd <<
        // 4`) envoyait un ID de commande invalide, silencieusement ignoré
        // par le Shell malgré un SendMessageW qui réussissait.
        let lparam = ((key.appcommand() | FAPPCOMMAND_KEY) as isize) << 16;
        SendMessageW(tray_hwnd, WM_APPCOMMAND, tray_hwnd as usize, lparam);
        true
    }
}
