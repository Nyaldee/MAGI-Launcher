pub mod calculator;
pub mod clipboard_history;
pub mod clock;
pub mod config;
pub mod disk_ejector;
pub mod emoji;
pub mod hotkey;
pub mod json_list;
pub mod launch;
pub mod media;
pub mod models;
pub mod recycle_bin;
pub mod search;
pub mod shortcuts;
pub mod supervisor;
pub mod timer;
pub mod windows;

/// Racine de chaque lecteur logique monté ("C:\\", "D:\\"...) -- partagée
/// par recycle_bin (qui parcourt le `$Recycle.Bin` de chaque lecteur) et
/// disk_ejector (qui n'en garde que la lettre). Liste vide si l'appel
/// échoue.
pub(crate) fn logical_drives() -> Vec<String> {
    unsafe {
        let mut buf = [0u16; 254];
        let len = crate::win32::kernel32::GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr());
        if len == 0 {
            return Vec::new();
        }
        let mut drives = Vec::new();
        let mut start = 0usize;
        for i in 0..len as usize {
            if buf[i] == 0 {
                if i > start {
                    drives.push(crate::win32::from_wstring(&buf[start..i]));
                }
                start = i + 1;
            }
        }
        drives
    }
}
