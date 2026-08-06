//! shell32.dll -- ShellExecute (lance les entrées de apps.json), requête/
//! vidage de la Corbeille, découpage de ligne de commande, icône de la
//! zone de notification (tray).

pub use windows_sys::Win32::UI::Shell::{
    CommandLineToArgvW, ShellExecuteExW, Shell_NotifyIconW, NOTIFYICONDATAW, NOTIFYICONDATAW_0, SHELLEXECUTEINFOW,
    SHELLEXECUTEINFOW_0, SHQUERYRBINFO, SHEmptyRecycleBinW, SHQueryRecycleBinW,
    // constantes
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS,
    SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
};
// SW_SHOWNORMAL vit sous le module "WindowsAndMessaging" de windows-sys
// (classement thématique, pas par DLL) mais reste la valeur nShow par
// défaut d'un appel ShellExecuteExW -- ré-exporté ici pour que
// core::launch n'ait pas besoin de savoir dans quel module windows-sys l'a
// rangée.
pub use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
