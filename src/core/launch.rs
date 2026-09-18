//! Lancement d'une entrée apps.json / révélation dans l'Explorateur.
//!
//! Tout passe par ShellExecuteExW (verbe par défaut du type de fichier),
//! jamais CreateProcess directement -- certaines cibles ne sont pas des
//! exécutables mais des documents ouverts via association de fichier (ex:
//! un `.msc` résolu en `mmc.exe`, décidé par Windows, pas par cette appli),
//! et seul ShellExecute sait faire cette résolution.

use crate::win32::combase::CoTaskMemFree;
use crate::win32::kernel32::LocalFree;
use crate::win32::shell32::{
    CommandLineToArgvW, ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    SHGetKnownFolderPath, SW_SHOWNORMAL, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music,
    FOLDERID_Pictures, FOLDERID_Videos, KF_FLAG_DEFAULT,
};
use crate::win32::user32::SW_HIDE;
use crate::win32::{last_error, to_wstring, wstrlen, GUID};

/// Dossiers "connus" de Windows individuellement redirigeables (Propriétés
/// > Emplacement dans l'Explorateur, ou OneDrive "Gérer la sauvegarde") :
/// `%USERPROFILE%\<un de ces noms>` ne vit pas forcément où sa
/// concaténation littérale le suggère. Liste FERMÉE -- un sous-dossier
/// arbitraire du profil (ex: `%USERPROFILE%\New folder`) n'a pas de GUID,
/// n'est jamais redirigeable, et sa concaténation littérale suffit.
const KNOWN_FOLDERS: &[(&str, GUID)] = &[
    ("Desktop", FOLDERID_Desktop),
    ("Documents", FOLDERID_Documents),
    ("Downloads", FOLDERID_Downloads),
    ("Music", FOLDERID_Music),
    ("Pictures", FOLDERID_Pictures),
    ("Videos", FOLDERID_Videos),
];

/// Chemin réel d'un dossier connu de Windows (`id` = une des GUID de
/// KNOWN_FOLDERS), PAS sa concaténation littérale sous `%USERPROFILE%` :
/// OneDrive (Known Folder Move) ou un déplacement manuel le redirigent
/// couramment ailleurs (autre lettre de lecteur, sous-dossier OneDrive...).
/// `None` si l'appel échoue -- l'appelant retombe alors sur la
/// concaténation littérale.
fn known_folder(id: &GUID) -> Option<String> {
    unsafe {
        let mut pwstr: *mut u16 = std::ptr::null_mut();
        let hr = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT as u32, std::ptr::null_mut(), &mut pwstr);
        if hr < 0 || pwstr.is_null() {
            return None;
        }
        let path = crate::win32::from_wstring(std::slice::from_raw_parts(pwstr, wstrlen(pwstr)));
        CoTaskMemFree(pwstr as *const core::ffi::c_void);
        Some(path)
    }
}

/// Si `rest` (en unités `char`, alignées sur la boucle de
/// `expand_env_vars`) commence par un séparateur suivi du composant de
/// chemin complet d'un des noms de KNOWN_FOLDERS (pas juste ce préfixe --
/// "Documentsfoo" ne compte pas), retourne sa GUID et le nombre de `char`
/// consommés (séparateur + nom).
fn strip_known_folder_component(rest: &[char]) -> Option<(GUID, usize)> {
    let (sep, after_sep) = rest.split_first()?;
    if *sep != '\\' && *sep != '/' {
        return None;
    }
    for (name, guid) in KNOWN_FOLDERS {
        // Ces noms sont ASCII : leur longueur en octets vaut leur longueur
        // en `char`.
        let len = name.len();
        let Some(head) = after_sep.get(..len) else { continue };
        let matches = head.iter().zip(name.chars()).all(|(a, b)| a.eq_ignore_ascii_case(&b));
        let tail_ok = after_sep.get(len).is_none_or(|c| *c == '\\' || *c == '/');
        if matches && tail_ok {
            return Some((*guid, 1 + len));
        }
    }
    None
}

/// Expansion façon Windows des `%VAR%` (aucun équivalent en stdlib Rust).
/// Un `%...%` inconnu/non terminé est
/// laissé tel quel, comme `ExpandEnvironmentStringsW` ; la recherche de
/// variable passe par `std::env::var`, qui sous Windows résout via
/// `GetEnvironmentVariableW` et est donc déjà insensible à la casse, comme
/// le bloc d'environnement lui-même. Cas particulier : `%USERPROFILE%\<dossier
/// connu>` (voir KNOWN_FOLDERS) est résolu via `known_folder` plutôt que par
/// la simple concaténation, pour survivre à une redirection.
pub fn expand_env_vars(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if let Some(rel_end) = chars[i + 1..].iter().position(|&c| c == '%') {
                let name: String = chars[i + 1..i + 1 + rel_end].iter().collect();
                if !name.is_empty() {
                    let after_var = i + 1 + rel_end + 1;
                    if name.eq_ignore_ascii_case("USERPROFILE") {
                        if let Some((guid, consumed)) = strip_known_folder_component(&chars[after_var..]) {
                            if let Some(folder) = known_folder(&guid) {
                                out.push_str(&folder);
                                i = after_var + consumed;
                                continue;
                            }
                        }
                    }
                    if let Ok(val) = std::env::var(&name) {
                        out.push_str(&val);
                        i = after_var;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn split_command_line(command: &str) -> Result<Vec<String>, String> {
    let wide = to_wstring(command);
    let mut argc: i32 = 0;
    unsafe {
        let argv = CommandLineToArgvW(wide.as_ptr(), &mut argc);
        if argv.is_null() {
            return Err(format!("échec de CommandLineToArgvW (erreur {})", last_error()));
        }
        let mut out = Vec::with_capacity(argc.max(0) as usize);
        for i in 0..argc as isize {
            let ptr = *argv.offset(i);
            let slice = std::slice::from_raw_parts(ptr, wstrlen(ptr));
            out.push(String::from_utf16_lossy(slice));
        }
        LocalFree(argv as *mut core::ffi::c_void);
        Ok(out)
    }
}

/// (cible, arguments) à partir d'un "path" de apps.json. Partagé avec
/// core::supervisor (résolution du nom d'exe à surveiller pour
/// l'auto-restart).
pub fn resolve_target(raw_path: &str) -> Result<(String, Vec<String>), String> {
    if raw_path.trim().is_empty() || raw_path.contains('\0') {
        // CommandLineToArgvW("") renvoie le chemin de l'exécutable du
        // PROCESSUS COURANT, pas une erreur : un "path" vide/blanc
        // relancerait donc silencieusement une seconde instance du lanceur.
        // Un octet NUL, lui, tronque la chaîne avant même d'atteindre
        // l'API. config.rs filtre déjà les paths vides d'apps.json, mais
        // resolve_target/launch sont aussi appelés avec des chemins moins
        // garantis (restart.json via core::supervisor).
        return Err("chemin invalide".to_string());
    }
    let expanded = expand_env_vars(raw_path);
    if std::path::Path::new(&expanded).exists() {
        // Chemin existant tel quel : pas d'arguments à extraire, et le
        // découper pour le réassembler à l'identique exposerait au piège
        // CommandLineToArgvW ci-dessus.
        return Ok((expanded, Vec::new()));
    }
    let mut parts = split_command_line(&expanded)?;
    if parts.is_empty() || parts[0].trim().is_empty() {
        return Err("cible vide".to_string());
    }
    let target = parts.remove(0);
    Ok((target, parts))
}

/// Re-quote les arguments pour ShellExecute, l'exacte opération inverse du
/// découpage de CommandLineToArgvW -- l'algorithme standard de
/// quoting argv du MSVCRT (les antislashs ne sont spéciaux que
/// immédiatement avant un `"`).
fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.chars().any(|c| c == ' ' || c == '\t' || c == '\n' || c == '\x0b' || c == '"') {
        return arg.to_string();
    }
    let mut result = String::from("\"");
    let mut backslashes: usize = 0;
    for c in arg.chars() {
        if c == '\\' {
            backslashes += 1;
        } else if c == '"' {
            result.push_str(&"\\".repeat(backslashes * 2 + 1));
            result.push('"');
            backslashes = 0;
        } else {
            result.push_str(&"\\".repeat(backslashes));
            backslashes = 0;
            result.push(c);
        }
    }
    result.push_str(&"\\".repeat(backslashes * 2));
    result.push('"');
    result
}

fn quote_args(args: &[String]) -> String {
    args.iter().map(|a| quote_arg(a)).collect::<Vec<_>>().join(" ")
}

/// Lance `raw_path` (un "path" de apps.json). `raw_cwd`, s'il est fourni,
/// est utilisé tel quel à la place du dossier de la cible (nécessaire pour
/// cmd.exe/powershell.exe, qui démarreraient sinon dans system32) ;
/// `hidden` démarre la cible sans fenêtre visible (SW_HIDE comme état
/// initial de la fenêtre, pas un style que le process s'appliquerait à
/// lui-même après avoir déjà flashé à l'écran).
pub fn launch(raw_path: &str, raw_cwd: Option<&str>, hidden: bool) -> Result<(), String> {
    let (target, args) = resolve_target(raw_path)?;
    let target_w = to_wstring(&target);
    let args_string = if args.is_empty() { None } else { Some(quote_args(&args)) };
    let args_w = args_string.as_ref().map(|s| to_wstring(s));

    let cwd = match raw_cwd {
        Some(c) => expand_env_vars(c),
        None => std::path::Path::new(&target).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
    };
    let cwd_w = if !cwd.is_empty() && std::path::Path::new(&cwd).is_dir() { Some(to_wstring(&cwd)) } else { None };

    let mut info = SHELLEXECUTEINFOW {
        // Convention Win32 "struct préfixée par sa taille" : cbSize DOIT
        // être renseigné, sinon ShellExecuteExW échoue avec
        // ERROR_INVALID_PARAMETER. Le Default de windows-sys se contente de
        // mettre la struct à zéro, d'où ce champ explicite.
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // SEE_MASK_FLAG_NO_UI : un échec de lancement (mauvais chemin,
        // cible manquante) est déjà signalé par notre propre `Err`. La
        // boîte "fichier introuvable" de Windows apparaîtrait sinon
        // par-dessus une popup toujours au premier plan, voire derrière
        // elle -- un dialogue inaccessible.
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
        // lpVerb à NULL (pas "open" en dur) : Windows choisit le verbe par
        // défaut du type de fichier, comme un double-clic dans
        // l'Explorateur. Certains types n'ont pas de verbe "open" (ex: .cpl,
        // qui n'a que "cplopen" et "runas") et échouent sinon avec
        // SE_ERR_NOASSOC.
        lpVerb: std::ptr::null(),
        lpFile: target_w.as_ptr(),
        lpParameters: args_w.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        lpDirectory: cwd_w.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        nShow: if hidden { SW_HIDE } else { SW_SHOWNORMAL },
        ..Default::default()
    };

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 {
        return Err(format!("impossible de lancer '{}' (erreur {})", raw_path, last_error()));
    }
    if !info.hProcess.is_null() {
        unsafe {
            crate::win32::kernel32::CloseHandle(info.hProcess);
        }
    }
    Ok(())
}

/// Révèle le dossier contenant `raw_path` dans l'Explorateur avec ce
/// fichier sélectionné (Maj+Entrée dans le lanceur) plutôt que de le
/// lancer. explorer.exe est ici un vrai exécutable, pas un document à
/// résoudre via association de fichier, donc c'est un vrai CreateProcess
/// (via std::process::Command), pas ShellExecute.
pub fn reveal_in_explorer(raw_path: &str) -> Result<(), String> {
    let (target, _args) = resolve_target(raw_path)?;
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", target))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etend_une_variable_environnement_connue() {
        let windir = std::env::var("windir").unwrap();
        assert_eq!(expand_env_vars("%windir%\\notepad.exe"), format!("{}\\notepad.exe", windir));
    }

    #[test]
    fn laisse_une_variable_inconnue_intacte() {
        assert_eq!(expand_env_vars("%totally_bogus_var_xyz%\\x"), "%totally_bogus_var_xyz%\\x");
    }

    #[test]
    fn userprofile_documents_utilise_le_dossier_connu() {
        // Pas de chemin en dur : on vérifie que %USERPROFILE%\Documents\...
        // se résout via le même dossier connu, y compris quand Documents
        // est redirigé (OneDrive, déplacement manuel).
        let documents = known_folder(&FOLDERID_Documents).expect("SHGetKnownFolderPath(FOLDERID_Documents) a échoué");
        assert_eq!(expand_env_vars("%USERPROFILE%\\Documents\\eternalsonata"), format!("{documents}\\eternalsonata"));
    }

    #[test]
    fn userprofile_reconnait_les_autres_dossiers_connus() {
        // Même mécanisme sur un autre dossier de KNOWN_FOLDERS : ce n'est
        // pas un cas spécial à "Documents".
        let downloads = known_folder(&FOLDERID_Downloads).expect("SHGetKnownFolderPath(FOLDERID_Downloads) a échoué");
        assert_eq!(expand_env_vars("%USERPROFILE%\\Downloads\\file.zip"), format!("{downloads}\\file.zip"));
    }

    #[test]
    fn userprofile_documentsfoo_nest_pas_confondu_avec_documents() {
        // "Documentsfoo" n'est pas le composant "Documents" : concaténation
        // littérale, pas le dossier connu.
        let userprofile = std::env::var("USERPROFILE").unwrap();
        assert_eq!(
            expand_env_vars("%USERPROFILE%\\Documentsfoo\\save"),
            format!("{userprofile}\\Documentsfoo\\save")
        );
    }

    #[test]
    fn userprofile_seul_reste_une_concatenation_litterale() {
        // "New folder" n'est pas un dossier de KNOWN_FOLDERS : aucune
        // redirection possible, concaténation littérale.
        let userprofile = std::env::var("USERPROFILE").unwrap();
        assert_eq!(expand_env_vars("%USERPROFILE%\\New folder"), format!("{userprofile}\\New folder"));
    }

    #[test]
    fn rejette_chemin_vide_ou_avec_nul() {
        assert!(resolve_target("").is_err());
        assert!(resolve_target("   ").is_err());
        assert!(resolve_target("a\0b").is_err());
    }

    #[test]
    fn decoupe_cible_et_arguments_quand_le_chemin_n_existe_pas() {
        let (target, args) = resolve_target("C:\\Some\\Nonexistent\\App.exe --flag value").unwrap();
        assert_eq!(target, "C:\\Some\\Nonexistent\\App.exe");
        assert_eq!(args, vec!["--flag".to_string(), "value".to_string()]);
    }

    #[test]
    fn quote_arg_entoure_les_arguments_avec_espaces() {
        assert_eq!(quote_arg("noSpaces"), "noSpaces");
        assert_eq!(quote_arg("has space"), "\"has space\"");
    }
}
