//! Requête/vidage/listing de la Corbeille via l'API Shell + lecture directe
//! du dossier `$Recycle.Bin` (pas d'énumération Shell/COM pour lister le
//! contenu : chaque fichier supprimé y a un `$I<hash>` -- ses métadonnées
//! (taille, date, chemin d'origine) dans un format binaire simple et
//! stable -- à côté d'un `$R<hash>` -- les données elles-mêmes).

use crate::win32::advapi32::{ConvertSidToStringSidW, GetTokenInformation, OpenProcessToken, TokenUser, TOKEN_QUERY,
                              TOKEN_USER};
use crate::win32::kernel32::{
    CloseHandle, FindClose, FindFirstFileW, FindNextFileW, GetCurrentProcess, GetLogicalDriveStringsW, LocalFree,
    INVALID_HANDLE_VALUE, FILE_ATTRIBUTE_DIRECTORY, WIN32_FIND_DATAW,
};
use crate::win32::shell32::{SHEmptyRecycleBinW, SHQueryRecycleBinW, SHQUERYRBINFO, SHERB_NOCONFIRMATION,
                             SHERB_NOPROGRESSUI, SHERB_NOSOUND};
use crate::win32::{from_wstring, to_wstring};

/// (nombre d'éléments, taille totale en octets) ; `(0, 0)` en cas d'échec
/// (une Corbeille vide se comporte pareil, pas besoin d'un chemin d'erreur
/// séparé ici).
pub fn query() -> (i64, i64) {
    let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
    let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
    if hr != 0 {
        return (0, 0);
    }
    (info.i64NumItems, info.i64Size)
}

fn empty_blocking() {
    unsafe {
        SHEmptyRecycleBinW(
            std::ptr::null_mut(),
            std::ptr::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        );
    }
}

/// SHEmptyRecycleBinW est un appel bloquant (I/O disque) -- ne doit jamais
/// tourner sur le thread UI (figerait la réactivité du hotkey/tray), donc
/// ceci le lance sur son propre thread ponctuel.
pub fn empty_async() {
    std::thread::spawn(empty_blocking);
}

#[derive(Debug, Clone)]
pub struct RecycleBinItem {
    /// Juste le nom de fichier/dossier, pour l'affichage dans la liste.
    pub name: String,
    /// Chemin complet du fichier `$I<hash>` (métadonnées) -- conservé pour
    /// pouvoir supprimer CET élément précis (voir delete_item), sans avoir
    /// à re-parcourir tout `$Recycle.Bin` pour le retrouver.
    pub info_path: String,
    /// Chemin complet du `$R<hash>` correspondant (les données elles-mêmes
    /// -- un fichier ou un dossier selon ce qui a été supprimé).
    pub data_path: String,
}

/// Liste chaque entrée `(nom, est_un_dossier)` d'un dossier correspondant
/// à `pattern` (ex: "*" ou "$I*") -- silencieux sur tout échec (dossier
/// inexistant ou inaccessible, ex: le `$Recycle.Bin` d'un autre
/// utilisateur) plutôt que de faire remonter une erreur pour un cas aussi
/// attendu.
fn list_dir(dir: &str, pattern: &str) -> Vec<(String, bool)> {
    let search = to_wstring(&format!("{}\\{}", dir.trim_end_matches('\\'), pattern));
    let mut data = WIN32_FIND_DATAW::default();
    let mut out = Vec::new();
    unsafe {
        let handle = FindFirstFileW(search.as_ptr(), &mut data);
        if handle == INVALID_HANDLE_VALUE {
            return out;
        }
        loop {
            let name = from_wstring(&data.cFileName);
            if name != "." && name != ".." {
                out.push((name, data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0));
            }
            if FindNextFileW(handle, &mut data) == 0 {
                break;
            }
        }
        FindClose(handle);
    }
    out
}

fn logical_drives() -> Vec<String> {
    unsafe {
        let mut buf = [0u16; 254];
        let len = GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr());
        if len == 0 {
            return Vec::new();
        }
        let mut drives = Vec::new();
        let mut start = 0usize;
        for i in 0..len as usize {
            if buf[i] == 0 {
                if i > start {
                    drives.push(from_wstring(&buf[start..i]));
                }
                start = i + 1;
            }
        }
        drives
    }
}

/// Décode un fichier `$I<hash>` -- format Windows 10/11 (version 2) :
/// 8 octets version (i64 LE), 8 octets taille d'origine (i64 LE), 8 octets
/// date de suppression (FILETIME, ignorée ici), 4 octets longueur du
/// chemin (i32 LE, en unités UTF-16 avec le NUL final), puis le chemin
/// d'origine en UTF-16LE. `None` si le fichier est trop court/corrompu ou
/// d'une version plus ancienne (pré-Windows 10) qu'on ne gère pas ici.
fn parse_info_file(path: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 28 {
        return None;
    }
    let version = i64::from_le_bytes(bytes[0..8].try_into().ok()?);
    if version < 2 {
        return None;
    }
    let path_bytes = &bytes[28..];
    let mut units = Vec::with_capacity(path_bytes.len() / 2);
    for chunk in path_bytes.chunks_exact(2) {
        let unit = u16::from_le_bytes([chunk[0], chunk[1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    if units.is_empty() {
        return None;
    }
    let original_path = String::from_utf16_lossy(&units);
    Some(original_path.rsplit(['\\', '/']).next().unwrap_or(&original_path).to_string())
}

/// SID (ex: "S-1-5-21-...-1001") de l'utilisateur courant -- chaque compte
/// a son propre sous-dossier `$Recycle.Bin\<SID>`, et celui-ci peut
/// contenir les dossiers d'autres comptes (ou d'anciens profils jamais
/// nettoyés) en plus du nôtre. `None` si le jeton/SID n'a pas pu être
/// résolu (cas quasi jamais atteint en pratique).
fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut needed: u32 = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed);
        let mut buf = vec![0u8; needed as usize];
        let ok = GetTokenInformation(token, TokenUser, buf.as_mut_ptr() as *mut _, needed, &mut needed) != 0;
        CloseHandle(token);
        if !ok {
            return None;
        }
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut sid_ptr: *mut u16 = std::ptr::null_mut();
        if ConvertSidToStringSidW(token_user.User.Sid, &mut sid_ptr) == 0 {
            return None;
        }
        let mut len = 0isize;
        while *sid_ptr.offset(len) != 0 {
            len += 1;
        }
        let sid_string = from_wstring(std::slice::from_raw_parts(sid_ptr, len as usize));
        LocalFree(sid_ptr as *mut core::ffi::c_void);
        Some(sid_string)
    }
}

/// Liste chaque élément actuellement dans la Corbeille DE L'UTILISATEUR
/// COURANT, toutes lettres de lecteur confondues -- appelé une seule fois
/// à l'entrée du mode dédié (voir Mode::RecycleBin dans ui::window), pas à
/// chaque frappe.
///
/// `$Recycle.Bin` contient un sous-dossier par SID utilisateur, pas
/// seulement celui de la session courante (autres comptes de la machine,
/// anciens profils jamais purgés...) -- sans ce filtre, cette liste
/// montrait TOUT ce qui traînait sur le disque plutôt que le contenu réel
/// de la Corbeille tel que Windows/l'Explorateur le montrent, avec des
/// listes bien plus grandes que ce que rapporte SHQueryRecycleBinW.
/// Plafond défensif -- ce mode sert à parcourir/rechercher au clavier, pas
/// à afficher une Corbeille de plusieurs milliers d'éléments d'un coup
/// (ex: suppression groupée de nombreux fichiers individuels) ; sans lui,
/// un tel cas ferait ramer/geler le lanceur le temps de tout lire sur
/// disque depuis le thread UI.
const MAX_ITEMS: usize = 2000;

pub fn list_items() -> Vec<RecycleBinItem> {
    let own_sid = current_user_sid();
    let mut items = Vec::new();
    'drives: for drive in logical_drives() {
        // `drive` se termine déjà par un antislash (ex: "D:\\", tel que
        // rendu par GetLogicalDriveStringsW) -- le retirer puis recoller
        // "$Recycle.Bin" directement donnait "D:$Recycle.Bin", un chemin
        // RELATIF au répertoire courant du lecteur D: (pas forcément sa
        // racine) plutôt qu'un chemin absolu, ce qui faisait disparaître
        // silencieusement tout lecteur autre que celui où le processus a
        // démarré (list_dir traite un dossier introuvable comme un
        // résultat vide, pas une erreur).
        let recycle_root = format!("{}$Recycle.Bin", drive);
        for (sid, is_dir) in list_dir(&recycle_root, "*") {
            if !is_dir {
                continue;
            }
            if let Some(own) = &own_sid {
                if !sid.eq_ignore_ascii_case(own) {
                    continue;
                }
            }
            let sid_path = format!("{}\\{}", recycle_root, sid);
            for (file_name, file_is_dir) in list_dir(&sid_path, "$I*") {
                if file_is_dir {
                    continue;
                }
                // Un `$I<hash>` orphelin (dont le `$R<hash>` -- les données
                // elles-mêmes -- n'existe plus) traîne parfois sur le
                // disque sans qu'il corresponde à un élément réellement
                // présent dans la Corbeille (index Shell désynchronisé,
                // fréquent sur les disques externes/partagés entre
                // plusieurs machines) -- l'Explorateur ne le montre pas non
                // plus, donc on l'ignore ici aussi.
                let r_name = format!("$R{}", &file_name[2..]);
                let data_path = format!("{}\\{}", sid_path, r_name);
                if !std::path::Path::new(&data_path).exists() {
                    continue;
                }
                let info_path = format!("{}\\{}", sid_path, file_name);
                if let Some(name) = parse_info_file(&info_path) {
                    items.push(RecycleBinItem { name, info_path, data_path });
                    if items.len() >= MAX_ITEMS {
                        break 'drives;
                    }
                }
            }
        }
    }
    items
}

/// Supprime définitivement UN SEUL élément de la Corbeille (ses fichiers
/// `$I<hash>`/`$R<hash>` directement sur disque) -- pas de SHFileOperation
/// (qui exigerait de reconstruire un IDList vers l'élément virtuel, un
/// poids inutile pour un cas aussi ciblé) : même principe de lecture
/// directe du dossier que list_items(). `$R<hash>` peut être un fichier ou
/// un dossier (élément supprimé récursivement) -- les deux cas sont gérés.
/// Certains éléments supprimés gardent leur attribut lecture seule
/// d'origine, qui bloquerait sinon la suppression -- retenté une fois après
/// l'avoir levé.
pub fn delete_item(item: &RecycleBinItem) -> bool {
    let data_path = std::path::Path::new(&item.data_path);
    let data_ok = if data_path.is_dir() {
        std::fs::remove_dir_all(data_path).is_ok()
    } else {
        remove_file_even_readonly(data_path)
    };
    let info_ok = remove_file_even_readonly(std::path::Path::new(&item.info_path));
    data_ok && info_ok
}

fn remove_file_even_readonly(path: &std::path::Path) -> bool {
    if std::fs::remove_file(path).is_ok() {
        return true;
    }
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut perms = metadata.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            if std::fs::set_permissions(path, perms).is_ok() {
                return std::fs::remove_file(path).is_ok();
            }
        }
    }
    false
}
