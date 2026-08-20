//! advapi32.dll -- jeton du process courant, utilisé uniquement pour
//! retrouver le SID de l'utilisateur courant (voir core::recycle_bin :
//! filtrer $Recycle.Bin\<SID> au bon sous-dossier, comme le fait
//! l'Explorateur, plutôt que de lire ceux de tous les comptes/anciens
//! profils présents sur le disque).

pub use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
pub use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
pub use windows_sys::Win32::System::Threading::OpenProcessToken;
