//! Éjection de périphériques de stockage EXTERNES (USB) par lettre de
//! lecteur, via accès direct au volume (`FSCTL_LOCK_VOLUME`/
//! `FSCTL_DISMOUNT_VOLUME` + `IOCTL_STORAGE_EJECT_MEDIA`).
//!
//! Volontairement PAS `CM_Request_Device_Eject` (le chemin du bac système
//! "Retirer le périphérique en toute sécurité") : cette API juge de
//! l'éjectabilité d'après un flag de capacité posé par le pilote, que
//! beaucoup de boîtiers USB-SATA/UASP bon marché ne posent pas
//! correctement -- l'option est alors absente ou grisée dans Windows. Le
//! chemin volume direct ne consulte pas ce flag : il verrouille/démonte la
//! lettre de lecteur elle-même, et marche donc dans ces cas-là aussi.

use crate::win32::kernel32::{
    BusTypeUsb, CloseHandle, CreateFileW, DeviceIoControl, GetVolumeInformationW, FSCTL_DISMOUNT_VOLUME,
    FSCTL_LOCK_VOLUME, FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE, IOCTL_STORAGE_EJECT_MEDIA,
    IOCTL_STORAGE_MEDIA_REMOVAL, IOCTL_STORAGE_QUERY_PROPERTY, INVALID_HANDLE_VALUE, OPEN_EXISTING,
    PropertyStandardQuery, PREVENT_MEDIA_REMOVAL, STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
    StorageDeviceProperty,
};
use crate::win32::{from_wstring, to_wstring};

#[derive(Debug, Clone)]
pub struct EjectableDrive {
    pub letter: char,
    /// Étiquette de volume ("SANDISK", "Backup"...) -- vide si le
    /// périphérique n'en a pas (courant sur les clés USB jamais renommées).
    pub label: String,
}

/// Ouvre le volume `letter:` en accès direct (`\\.\D:`, pas `D:\`) :
/// nécessaire pour `DeviceIoControl`, un chemin de dossier classique
/// n'exposant aucun de ces IOCTL. `desired_access = 0` suffit pour
/// interroger les propriétés (`is_usb_drive`) ; verrouiller/démonter exige
/// GENERIC_READ|GENERIC_WRITE.
fn open_volume(letter: char, desired_access: u32) -> Option<crate::win32::HANDLE> {
    let path = to_wstring(&format!("\\\\.\\{letter}:"));
    unsafe {
        let handle =
            CreateFileW(path.as_ptr(), desired_access, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut());
        if handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(handle)
        }
    }
}

/// `true` si le volume est porté par un bus USB. `GetDriveTypeW` ne suffit
/// pas : beaucoup de disques externes (boîtiers USB-SATA/UASP) s'annoncent
/// `DRIVE_FIXED` et non `DRIVE_REMOVABLE`. `IOCTL_STORAGE_QUERY_PROPERTY`
/// donne le vrai type de bus, ce qui garantit de ne lister que les
/// périphériques réellement externes (jamais un disque interne).
fn is_usb_drive(letter: char) -> bool {
    let Some(handle) = open_volume(letter, 0) else { return false };
    unsafe {
        let query =
            STORAGE_PROPERTY_QUERY { PropertyId: StorageDeviceProperty, QueryType: PropertyStandardQuery, AdditionalParameters: [0] };
        let mut descriptor: STORAGE_DEVICE_DESCRIPTOR = std::mem::zeroed();
        let mut bytes_returned: u32 = 0;
        let ok = DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            &mut descriptor as *mut _ as *mut core::ffi::c_void,
            std::mem::size_of::<STORAGE_DEVICE_DESCRIPTOR>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        ) != 0;
        CloseHandle(handle);
        ok && descriptor.BusType == BusTypeUsb
    }
}

/// Étiquette de volume via `GetVolumeInformationW` -- chaîne vide si
/// absente/inaccessible, jamais une erreur : une clé USB sans nom est un
/// cas normal.
fn volume_label(letter: char) -> String {
    let root = to_wstring(&format!("{letter}:\\"));
    let mut buf = [0u16; 128];
    unsafe {
        let ok = GetVolumeInformationW(
            root.as_ptr(),
            buf.as_mut_ptr(),
            buf.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        ) != 0;
        if ok {
            from_wstring(&buf)
        } else {
            String::new()
        }
    }
}

/// `DeviceIoControl` avec un buffer de sortie toujours nul -- le moule des
/// quatre appels d'`eject_drive`. `is_usb_drive` attend un descripteur en
/// sortie et reste donc en dehors.
unsafe fn ioctl(handle: crate::win32::HANDLE, code: u32, input: *const core::ffi::c_void, input_size: u32) -> bool {
    let mut bytes_returned: u32 = 0;
    DeviceIoControl(handle, code, input, input_size, std::ptr::null_mut(), 0, &mut bytes_returned, std::ptr::null_mut()) != 0
}

pub fn list_ejectable_drives() -> Vec<EjectableDrive> {
    super::logical_drives()
        .iter()
        .filter_map(|drive| drive.chars().next())
        .filter(|&letter| is_usb_drive(letter))
        .map(|letter| EjectableDrive { letter, label: volume_label(letter) })
        .collect()
}

/// Verrouille, démonte puis éjecte le volume -- `true` seulement si
/// l'éjection finale a réussi.
///
/// `FSCTL_LOCK_VOLUME` échoue précisément quand un autre process détient
/// encore un handle sur le volume : signal fiable, et déjà gratuit puisque
/// l'appel a lieu de toute façon. `force` décide de la suite :
/// - `false` (Entrée) : verrou refusé -> abandon ICI, AVANT tout
///   démontage, le disque n'est pas touché.
/// - `true` (Maj+Suppr) : le refus est ignoré et le démontage a lieu quand
///   même -- `FSCTL_DISMOUNT_VOLUME` ne dépend pas du verrou préalable, il
///   invalide les handles ouverts au lieu d'attendre leur libération. Une
///   lecture en cours (antivirus, indexeur) échoue proprement ; une
///   ÉCRITURE en cours finira tronquée.
pub fn eject_drive(letter: char, force: bool) -> bool {
    let Some(handle) = open_volume(letter, GENERIC_READ | GENERIC_WRITE) else { return false };
    unsafe {
        let locked = ioctl(handle, FSCTL_LOCK_VOLUME, std::ptr::null(), 0);
        if !locked && !force {
            CloseHandle(handle);
            return false;
        }
        ioctl(handle, FSCTL_DISMOUNT_VOLUME, std::ptr::null(), 0);
        let prevent = PREVENT_MEDIA_REMOVAL { PreventMediaRemoval: false };
        ioctl(
            handle,
            IOCTL_STORAGE_MEDIA_REMOVAL,
            &prevent as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<PREVENT_MEDIA_REMOVAL>() as u32,
        );
        let ejected = ioctl(handle, IOCTL_STORAGE_EJECT_MEDIA, std::ptr::null(), 0);
        CloseHandle(handle);
        ejected
    }
}
