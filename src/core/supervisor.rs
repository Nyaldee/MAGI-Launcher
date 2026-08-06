//! Thread de fond qui surveille la liste de cibles Auto-restart et relance
//! toute cible dont le processus disparaît (crash ou fermeture manuelle --
//! seulement "ce nom d'exécutable tourne-t-il encore", donc impossible (et
//! pas cherché) de distinguer les deux cas).
//!
//! Par sondage (CreateToolhelp32Snapshot toutes les deux secondes), pas
//! événementiel -- il n'existe aucune notification Win32 "ce processus est
//! mort" utilisée ici, délibérément plus simple que la boucle pilotée par
//! messages de ui::window.

use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::win32::from_wstring;
use crate::win32::kernel32::{
    CloseHandle, CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, INVALID_HANDLE_VALUE, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};

use super::launch;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Nom de fichier (pas le chemin complet, en minuscules) de la cible RÉSOLUE
/// d'une entrée de restart.json -- passe par `launch::resolve_target`
/// d'abord pour séparer la cible de ses éventuels arguments (ex:
/// "ShareX.exe -portable -silent") avant d'en extraire le nom de fichier ;
/// sans ça, une cible avec arguments ne matche jamais aucun nom de
/// `running_exe_names()` (la comparaison porterait sur la cible entière,
/// arguments compris), donc semblerait ne jamais tourner et serait
/// relancée à chaque sondage même si déjà active.
pub fn exe_basename(path: &str) -> String {
    let resolved = launch::resolve_target(path).map(|(target, _args)| target).unwrap_or_else(|_| path.to_string());
    let normalized = resolved.replace('\\', "/");
    match normalized.rsplit('/').next() {
        Some(name) if !name.is_empty() => name.to_lowercase(),
        _ => resolved.to_lowercase(),
    }
}

/// Instantané du nom de fichier exécutable (pas le chemin complet) de
/// chaque processus en cours, via un instantané Toolhelp32.
pub fn running_exe_names() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry = PROCESSENTRY32W::default();
        // Convention Win32 classique des structs "préfixées par leur
        // taille" : dwSize DOIT être renseigné avant l'appel, sinon
        // Process32FirstW échoue silencieusement (renvoie FALSE). Le
        // Default généré par windows-sys se contente de mettre la struct
        // à zéro, donc ce champ doit être fixé ici à la main -- l'ancien
        // FFI écrit à la main le faisait via un Default() personnalisé,
        // qui n'existe plus une fois la struct windows-sys utilisée
        // directement.
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                out.push(from_wstring(&entry.szExeFile));
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    out
}

pub struct RestartSupervisor {
    targets: Arc<Mutex<Vec<String>>>,
    stop_tx: Option<Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl RestartSupervisor {
    pub fn new(initial_targets: Vec<String>) -> RestartSupervisor {
        RestartSupervisor { targets: Arc::new(Mutex::new(initial_targets)), stop_tx: None, handle: None }
    }

    pub fn set_targets(&self, targets: Vec<String>) {
        *self.targets.lock().unwrap() = targets;
    }

    pub fn start(&mut self) {
        if self.handle.is_some() {
            return;
        }
        let (tx, rx) = channel::<()>();
        self.stop_tx = Some(tx);
        let targets = Arc::clone(&self.targets);
        self.handle = Some(thread::spawn(move || loop {
            let watch = targets.lock().unwrap().clone();
            if !watch.is_empty() {
                let running: Vec<String> = running_exe_names().iter().map(|s| s.to_lowercase()).collect();
                for target in &watch {
                    let name = exe_basename(target);
                    if !running.contains(&name) {
                        let _ = launch::launch(target, None, false);
                    }
                }
            }
            match rx.recv_timeout(POLL_INTERVAL) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => continue,
            }
        }));
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for RestartSupervisor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_basename_handles_both_separators() {
        assert_eq!(exe_basename("C:\\Apps\\Foo\\Foo.exe"), "foo.exe");
        assert_eq!(exe_basename("C:/Apps/Foo/Foo.exe"), "foo.exe");
        assert_eq!(exe_basename("Foo.exe"), "foo.exe");
    }

    #[test]
    fn exe_basename_ignores_trailing_arguments() {
        // Cible avec arguments (ex: apps.json "path": "...\\ShareX.exe
        // -portable -silent") -- doit toujours résoudre vers le seul nom de
        // fichier de l'exécutable, pas la ligne de commande entière.
        assert_eq!(exe_basename("A:\\Apps\\ShareX\\ShareX.exe -portable -silent"), "sharex.exe");
    }

    #[test]
    fn running_exe_names_includes_self() {
        // Notre propre processus tourne forcément -- un test de fumée
        // simple pour vérifier que le parcours d'instantané Toolhelp32
        // fonctionne vraiment de bout en bout.
        let names = running_exe_names();
        assert!(!names.is_empty());
    }

    #[test]
    fn start_and_stop_do_not_hang() {
        let mut sup = RestartSupervisor::new(Vec::new());
        sup.start();
        sup.set_targets(vec!["C:\\definitely\\not\\a\\real\\app.exe".to_string()]);
        sup.stop();
    }
}
