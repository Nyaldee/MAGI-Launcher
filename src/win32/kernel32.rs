//! kernel32.dll -- processus/threads/snapshots, formatage d'heure locale,
//! le mutex mono-instance.

pub use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, LocalFree, ERROR_ALREADY_EXISTS,
                                          INVALID_HANDLE_VALUE, SYSTEMTIME};
pub use windows_sys::Win32::Globalization::{GetTimeFormatEx, TIME_NOSECONDS};
pub use windows_sys::Win32::System::SystemInformation::GetLocalTime;
pub use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
pub use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
pub use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, GetCurrentThreadId, OpenProcess, ReleaseMutex, TerminateProcess,
    PROCESS_TERMINATE,
};
pub use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
pub use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileW, FindNextFileW, GetLogicalDriveStringsW, FILE_ATTRIBUTE_DIRECTORY, WIN32_FIND_DATAW,
};
// Utilisé uniquement par le stress test (mémoire du process) -- jamais
// appelé par l'appli elle-même, donc sans impact sur le binaire livré.
pub use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
