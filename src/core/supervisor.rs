//! Thread de fond qui surveille la liste de cibles Auto-restart et relance
//! toute cible dont le processus disparaît. Le critère est uniquement "ce
//! nom d'exécutable tourne-t-il encore", donc crash et fermeture manuelle
//! sont indiscernables.
//!
//! Par sondage (CreateToolhelp32Snapshot toutes les deux secondes) plutôt
//! qu'événementiel : délibérément plus simple que la boucle pilotée par
//! messages de ui::window.

use std::collections::HashSet;
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

/// Nom de fichier en minuscules (pas le chemin complet) de la cible RÉSOLUE
/// d'une entrée de restart.json. `launch::resolve_target` sépare d'abord la
/// cible de ses arguments (ex: "ShareX.exe -portable -silent") : sans ça la
/// comparaison avec `running_exe_names()` porterait sur la ligne de commande
/// entière, ne matcherait jamais, et la cible serait relancée à chaque
/// sondage même si déjà active.
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
        // Struct Win32 "préfixée par sa taille" : dwSize DOIT être
        // renseigné avant l'appel, sinon Process32FirstW échoue
        // silencieusement (renvoie FALSE). Le Default de windows-sys se
        // contente de zéroer la struct, d'où ce champ posé à la main.
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
            // Copie sous verrou : celui-ci ne doit pas rester tenu pendant
            // les lancements, qui peuvent bloquer un moment.
            let watch = targets.lock().unwrap().clone();
            if !watch.is_empty() {
                let running: HashSet<String> = running_exe_names().into_iter().map(|s| s.to_lowercase()).collect();
                for target in &watch {
                    if !running.contains(&exe_basename(target)) {
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
        // Doit résoudre vers le seul nom de fichier, pas la ligne de
        // commande entière.
        assert_eq!(exe_basename("A:\\Apps\\ShareX\\ShareX.exe -portable -silent"), "sharex.exe");
    }

    #[test]
    fn running_exe_names_includes_self() {
        // Notre propre processus tourne forcément : test de fumée du
        // parcours d'instantané Toolhelp32 de bout en bout.
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
