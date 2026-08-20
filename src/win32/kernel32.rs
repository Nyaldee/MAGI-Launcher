//! kernel32.dll -- processus/threads/snapshots, formatage d'heure locale,
//! le mutex mono-instance.

pub use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, LocalFree, ERROR_ALREADY_EXISTS, GENERIC_READ,
                                          GENERIC_WRITE, INVALID_HANDLE_VALUE, SYSTEMTIME};
pub use windows_sys::Win32::Globalization::{GetTimeFormatEx, TIME_NOSECONDS};
pub use windows_sys::Win32::System::SystemInformation::GetLocalTime;
pub use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
pub use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
// AttachThreadInput est exportée par user32.dll, pas kernel32.dll :
// windows-sys la range sous System::Threading, par thème et non par DLL
// (même situation qu'en tête de gdi32.rs). Son usage : ui::window,
// start_bounce.
pub use windows_sys::Win32::System::Threading::{
    AttachThreadInput, CreateMutexW, GetCurrentProcess, GetCurrentThreadId, OpenProcess, TerminateProcess,
    PROCESS_TERMINATE,
};
pub use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, VirtualLock, VirtualUnlock};
pub use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FindClose, FindFirstFileW, FindNextFileW, GetLogicalDriveStringsW, GetVolumeInformationW,
    BusTypeUsb, FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, WIN32_FIND_DATAW,
};
// DeviceIoControl (kernel32.dll) + les IOCTL de stockage utilisés par
// core::disk_ejector -- voir son commentaire d'en-tête pour pourquoi ce
// chemin (volume direct) plutôt que CM_Request_Device_Eject.
pub use windows_sys::Win32::System::Ioctl::{
    FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_STORAGE_EJECT_MEDIA, IOCTL_STORAGE_MEDIA_REMOVAL,
    IOCTL_STORAGE_QUERY_PROPERTY, PREVENT_MEDIA_REMOVAL, PropertyStandardQuery, StorageDeviceProperty,
    STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
};
pub use windows_sys::Win32::System::IO::DeviceIoControl;
// Mémoire du process, utilisée par le seul stress test : gated cfg(test)
// pour ne pas apparaître dans le binaire livré.
#[cfg(test)]
pub use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
