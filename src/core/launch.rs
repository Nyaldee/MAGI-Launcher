//! Lancement d'une entrée apps.json / révélation dans l'Explorateur.
//!
//! Tout passe par ShellExecuteExW (verbe "open"), jamais CreateProcess
//! directement -- certaines cibles ne sont pas des exécutables mais des
//! documents ouverts via association de fichier (ex: un `.msc` résolu en
//! `mmc.exe`, décidé par Windows, pas par cette appli), et seul
//! ShellExecute sait faire cette résolution.

use crate::win32::kernel32::LocalFree;
use crate::win32::shell32::{
    CommandLineToArgvW, ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    SW_SHOWNORMAL,
};
use crate::win32::user32::SW_HIDE;
use crate::win32::{last_error, to_wstring};

/// Expansion façon Windows des `%VAR%` (pas d'équivalent stdlib à
/// `os.path.expandvars` en Rust). Un `%...%` inconnu/non terminé est
/// laissé tel quel, comme `ExpandEnvironmentStringsW` ; la recherche de
/// variable passe par `std::env::var`, qui sous Windows résout via
/// `GetEnvironmentVariableW` et est donc déjà insensible à la casse, comme
/// le bloc d'environnement lui-même.
pub fn expand_env_vars(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if let Some(rel_end) = chars[i + 1..].iter().position(|&c| c == '%') {
                let name: String = chars[i + 1..i + 1 + rel_end].iter().collect();
                if !name.is_empty() {
                    if let Ok(val) = std::env::var(&name) {
                        out.push_str(&val);
                        i += rel_end + 2;
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
            let mut len = 0isize;
            while *ptr.offset(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len as usize);
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
        // CommandLineToArgvW("") a un comportement documenté mais
        // surprenant : il renvoie le chemin de l'exécutable du PROCESSUS
        // COURANT lui-même, pas une erreur -- un "path" vide/blanc
        // relancerait donc silencieusement une seconde instance du
        // lanceur. Un octet NUL tronque silencieusement la chaîne avant
        // même d'atteindre l'API. Aucun des deux n'est atteignable depuis
        // apps.json (config.rs filtre déjà les paths vides), mais
        // resolve_target/launch sont aussi appelés avec des chemins moins
        // garantis (ex: restart.json via core::supervisor).
        return Err("chemin invalide".to_string());
    }
    let expanded = expand_env_vars(raw_path);
    if std::path::Path::new(&expanded).exists() {
        // Un chemin simple sans arguments -- inutile (et, vu le piège
        // CommandLineToArgvW ci-dessus, un peu risqué) de le découper pour
        // le réassembler à l'identique ensuite.
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

    let verb_w = to_wstring("open");
    let mut info = SHELLEXECUTEINFOW {
        // Convention Win32 "struct préfixée par sa taille" : cbSize DOIT
        // être renseigné, sinon ShellExecuteExW échoue silencieusement
        // (ERROR_INVALID_PARAMETER). Le Default généré par windows-sys ne
        // fait que mettre la struct à zéro -- ce champ doit donc être fixé
        // explicitement ici (l'ancien FFI écrit à la main le faisait via
        // un Default() personnalisé, qui n'existe plus avec la struct
        // windows-sys utilisée directement).
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // SEE_MASK_FLAG_NO_UI : un lancement en échec (mauvais chemin,
        // cible manquante) est déjà signalé via notre propre `Err` --
        // Windows ne doit jamais afficher sa propre boîte "fichier
        // introuvable" par-dessus une popup toujours au premier plan (ou,
        // pire, derrière elle -- exactement le risque de dialogue piégé
        // que l'ordre "cacher avant de lancer" de main.rs évite).
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI,
        lpVerb: verb_w.as_ptr(),
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
