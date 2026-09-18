//! Thread de fond unique qui sonde la liste des processus toutes les deux
//! secondes (`CreateToolhelp32Snapshot`) et pilote DEUX comportements à
//! partir du même instantané :
//!   - **Auto-restart** : relance toute cible de `restart.json` dont le
//!     processus a disparu (crash et fermeture manuelle sont indiscernables,
//!     le seul critère est "ce nom d'exécutable tourne-t-il encore").
//!   - **Auto-kill** : termine toute cible de `kill.json` qui tourne, APRÈS
//!     un délai de grâce fixe de 10 s à chaque (ré)activation -- le temps
//!     d'ouvrir le menu du tray et de désactiver la fonction si `kill.json`
//!     a été mal rempli (voir `KILL_GRACE`). Ne se termine JAMAIS lui-même
//!     (voir `self_exe`) : sinon un `kill.json` fautif + un lancement au
//!     démarrage tuerait MAGI à chaque boot, sans tray pour l'arrêter.
//!
//! Par sondage plutôt qu'événementiel : délibérément plus simple que la
//! boucle pilotée par messages de ui::window. Un seul instantané par tour
//! sert les deux besoins.

use std::collections::HashSet;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::win32::from_wstring;
use crate::win32::kernel32::{
    CloseHandle, CreateToolhelp32Snapshot, OpenProcess, Process32FirstW, Process32NextW, TerminateProcess,
    INVALID_HANDLE_VALUE, PROCESSENTRY32W, PROCESS_TERMINATE, TH32CS_SNAPPROCESS,
};

use super::launch;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Délai imposé, non configurable, entre l'activation d'Auto-kill (lancement
/// de MAGI ou bascule du tray) et le premier kill : de quoi désactiver la
/// fonction depuis le tray si `kill.json` vise un composant nécessaire.
const KILL_GRACE: Duration = Duration::from_secs(10);

/// Nom de fichier en minuscules (pas le chemin complet) de la cible RÉSOLUE
/// d'une entrée de restart.json / kill.json. `launch::resolve_target` sépare
/// d'abord la cible de ses arguments (ex: "ShareX.exe -portable -silent") :
/// sans ça la comparaison avec la liste des process porterait sur la ligne
/// de commande entière, ne matcherait jamais.
pub fn exe_basename(path: &str) -> String {
    let resolved = launch::resolve_target(path).map(|(target, _args)| target).unwrap_or_else(|_| path.to_string());
    let normalized = resolved.replace('\\', "/");
    match normalized.rsplit('/').next() {
        Some(name) if !name.is_empty() => name.to_lowercase(),
        _ => resolved.to_lowercase(),
    }
}

/// Instantané `(nom de fichier exécutable en minuscules, PID)` de chaque
/// processus en cours, via un instantané Toolhelp32.
pub fn running_processes() -> Vec<(String, u32)> {
    let mut out = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry = PROCESSENTRY32W::default();
        // Struct Win32 "préfixée par sa taille" : dwSize DOIT être renseigné
        // avant l'appel, sinon Process32FirstW échoue silencieusement.
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                out.push((from_wstring(&entry.szExeFile).to_lowercase(), entry.th32ProcessID));
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    out
}

fn terminate_pid(pid: u32) {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !handle.is_null() {
            TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}

/// Basename minuscule de l'exécutable en cours -- exclu d'Auto-kill.
fn self_exe_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_else(|| "magi_launcher.exe".to_string())
}

/// Cible d'une liste, avec son basename d'exécutable RÉSOLU une seule fois
/// (à `set_*_targets`) plutôt qu'à chaque tour de sondage : `exe_basename`
/// appelle `resolve_target`, qui peut toucher le disque.
#[derive(Clone)]
struct Target {
    raw: String,
    base: String,
}

impl Target {
    fn resolve(raw: String) -> Target {
        let base = exe_basename(&raw);
        Target { raw, base }
    }
}

struct Inner {
    restart_targets: Mutex<Vec<Target>>,
    kill_targets: Mutex<Vec<Target>>,
    restart_enabled: AtomicBool,
    kill_enabled: AtomicBool,
    /// `Some(instant)` = début du délai de grâce Auto-kill en cours ; `None`
    /// quand Auto-kill est désactivé. Aucun kill tant que
    /// `elapsed() < KILL_GRACE`.
    kill_armed_at: Mutex<Option<Instant>>,
    self_exe: String,
    /// Dernier instantané des noms d'exécutables en cours (minuscules), posé
    /// à chaque tour où le sondage a lieu. Lu par l'UI (marqueurs ★/☆ et
    /// ✖/· des pickers) pour éviter un `CreateToolhelp32Snapshot` synchrone
    /// sur le thread d'affichage. Amorcé au démarrage par `new`.
    last_running: Mutex<HashSet<String>>,
}

/// Superviseur de processus unique (Auto-restart + Auto-kill). Le thread ne
/// tourne qu'entre `start()` et `stop()` ; les deux comportements s'activent/
/// se coupent indépendamment via `set_*_enabled`, sans toucher au thread.
pub struct ProcessSupervisor {
    inner: Arc<Inner>,
    stop_tx: Option<Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl ProcessSupervisor {
    pub fn new(restart_targets: Vec<String>, kill_targets: Vec<String>) -> ProcessSupervisor {
        // Un instantané au démarrage : la boucle ne le rafraîchit ensuite que
        // les tours où le sondage a lieu (voir Inner::last_running), or l'UI
        // peut ouvrir un picker avant le premier tour du thread.
        let last_running = running_processes().into_iter().map(|(name, _pid)| name).collect();
        ProcessSupervisor {
            inner: Arc::new(Inner {
                restart_targets: Mutex::new(restart_targets.into_iter().map(Target::resolve).collect()),
                kill_targets: Mutex::new(kill_targets.into_iter().map(Target::resolve).collect()),
                restart_enabled: AtomicBool::new(false),
                kill_enabled: AtomicBool::new(false),
                kill_armed_at: Mutex::new(None),
                self_exe: self_exe_name(),
                last_running: Mutex::new(last_running),
            }),
            stop_tx: None,
            handle: None,
        }
    }

    pub fn set_restart_targets(&self, targets: Vec<String>) {
        // Résolution HORS verrou (peut toucher le disque).
        let resolved: Vec<Target> = targets.into_iter().map(Target::resolve).collect();
        *self.inner.restart_targets.lock().unwrap() = resolved;
    }

    pub fn set_kill_targets(&self, targets: Vec<String>) {
        let resolved: Vec<Target> = targets.into_iter().map(Target::resolve).collect();
        *self.inner.kill_targets.lock().unwrap() = resolved;
    }

    /// Dernier instantané des noms d'exécutables en cours (minuscules) --
    /// pour l'UI, qui ne refait pas de `CreateToolhelp32Snapshot` de son côté.
    /// Au plus `POLL_INTERVAL` de retard, et jamais rafraîchi tant qu'aucune
    /// facette n'est active (les marqueurs sont purement décoratifs).
    pub fn running_names_snapshot(&self) -> HashSet<String> {
        self.inner.last_running.lock().unwrap().clone()
    }

    pub fn set_restart_enabled(&self, enabled: bool) {
        self.inner.restart_enabled.store(enabled, Ordering::Relaxed);
    }

    /// (Ré)arme le délai de grâce à chaque passage à `true` : chaque
    /// activation -- lancement de MAGI compris -- laisse `KILL_GRACE` avant
    /// le premier kill.
    pub fn set_kill_enabled(&self, enabled: bool) {
        self.inner.kill_enabled.store(enabled, Ordering::Relaxed);
        *self.inner.kill_armed_at.lock().unwrap() = enabled.then(Instant::now);
    }

    pub fn start(&mut self) {
        if self.handle.is_some() {
            return;
        }
        let (tx, rx) = channel::<()>();
        self.stop_tx = Some(tx);
        let inner = Arc::clone(&self.inner);
        self.handle = Some(thread::spawn(move || loop {
            let restart_on = inner.restart_enabled.load(Ordering::Relaxed);
            let kill_on = inner.kill_enabled.load(Ordering::Relaxed);
            // Aucune facette active : rien à faire, pas de snapshot.
            let procs = if restart_on || kill_on { running_processes() } else { Vec::new() };
            if restart_on || kill_on {
                *inner.last_running.lock().unwrap() = procs.iter().map(|(name, _)| name.clone()).collect();
            }

            if restart_on {
                // Copie sous verrou : ne pas le tenir pendant les lancements,
                // qui peuvent bloquer un moment.
                let watch = inner.restart_targets.lock().unwrap().clone();
                if !watch.is_empty() {
                    let running: HashSet<&str> = procs.iter().map(|(name, _)| name.as_str()).collect();
                    for target in &watch {
                        if !running.contains(target.base.as_str()) {
                            let _ = launch::launch(&target.raw, None, false);
                        }
                    }
                }
            }

            if kill_on {
                let past_grace = inner.kill_armed_at.lock().unwrap().is_some_and(|t| t.elapsed() >= KILL_GRACE);
                if past_grace {
                    let watch = inner.kill_targets.lock().unwrap().clone();
                    for target in &watch {
                        if target.base == inner.self_exe {
                            continue;
                        }
                        for (name, pid) in &procs {
                            if *name == target.base {
                                terminate_pid(*pid);
                            }
                        }
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

impl Drop for ProcessSupervisor {
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
        assert_eq!(exe_basename("A:\\Apps\\ShareX\\ShareX.exe -portable -silent"), "sharex.exe");
    }

    #[test]
    fn running_processes_returns_named_entries_including_self() {
        let procs = running_processes();
        assert!(!procs.is_empty());
        // Le nom est toujours renseigné et en minuscules ; notre propre
        // process figure forcément dans l'instantané.
        assert!(procs.iter().all(|(name, _pid)| !name.is_empty() && name == &name.to_lowercase()));
        let self_pid = std::process::id();
        assert!(procs.iter().any(|(_name, pid)| *pid == self_pid));
    }

    #[test]
    fn self_exe_is_the_resolved_running_binary_basename() {
        // `inner.self_exe` (jamais tué par Auto-kill) doit correspondre au
        // basename résolu de l'exécutable en cours -- c'est ce qui protège
        // MAGI d'un kill.json qui le viserait.
        let sup = ProcessSupervisor::new(Vec::new(), Vec::new());
        let running_self = std::env::current_exe().unwrap().to_string_lossy().to_string();
        assert_eq!(sup.inner.self_exe, exe_basename(&running_self));
    }

    #[test]
    fn start_and_stop_do_not_hang() {
        let mut sup = ProcessSupervisor::new(Vec::new(), Vec::new());
        sup.start();
        sup.set_restart_targets(vec!["C:\\definitely\\not\\a\\real\\app.exe".to_string()]);
        sup.set_kill_enabled(true);
        sup.set_kill_enabled(false);
        sup.stop();
    }
}
