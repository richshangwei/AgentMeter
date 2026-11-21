use serde::Serialize;
use std::sync::Mutex;
use windows_sys::Win32::{
    Foundation::FILETIME,
    Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives},
    System::{
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetSystemTimes,
    },
};

#[derive(Default)]
pub struct ResourceState(Mutex<Option<CpuTimes>>);
#[derive(Clone, Copy)]
struct CpuTimes {
    idle: u64,
    kernel: u64,
    user: u64,
}
#[derive(Serialize)]
pub struct Capacity {
    used: u64,
    total: u64,
}
#[derive(Serialize)]
pub struct Disk {
    path: String,
    used: u64,
    total: u64,
}
#[derive(Serialize)]
pub struct Resources {
    cpu_error: bool,
    cpu_percent: Option<f64>,
    memory: Option<Capacity>,
    disk: Option<Capacity>,
    disk_path: String,
    disks: Vec<Disk>,
}

const DRIVE_FIXED: u32 = 3;

/// Drive roots such as `C:\` for every bit set in a `GetLogicalDrives` mask.
fn drive_roots(mask: u32) -> Vec<String> {
    (0..26u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| format!("{}:\\", char::from(b'A' + bit)))
        .collect()
}

fn capacity_of(root: &std::ffi::OsStr) -> Option<Capacity> {
    use std::os::windows::ffi::OsStrExt;
    if root.is_empty() {
        return None;
    }
    let wide: Vec<u16> = root.encode_wide().chain(std::iter::once(0)).collect();
    let mut total = 0;
    let mut free = 0;
    let ok =
        unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), std::ptr::null_mut(), &mut total, &mut free) }
            != 0;
    if !ok || total == 0 {
        return None;
    }
    total.checked_sub(free).map(|used| Capacity { used, total })
}

/// Local fixed drives only: network shares and removable media can block or prompt.
fn fixed_disks() -> Vec<Disk> {
    let mask = unsafe { GetLogicalDrives() };
    drive_roots(mask)
        .into_iter()
        .filter_map(|root| {
            let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            if unsafe { GetDriveTypeW(wide.as_ptr()) } != DRIVE_FIXED {
                return None;
            }
            let capacity = capacity_of(std::ffi::OsStr::new(&root))?;
            Some(Disk {
                path: root.trim_end_matches('\\').to_owned(),
                used: capacity.used,
                total: capacity.total,
            })
        })
        .collect()
}

fn cpu_usage(previous: CpuTimes, current: CpuTimes) -> Option<f64> {
    let idle = current.idle.checked_sub(previous.idle)?;
    let total = current
        .kernel
        .checked_sub(previous.kernel)?
        .checked_add(current.user.checked_sub(previous.user)?)?;
    if total == 0 || idle > total {
        return None;
    }
    Some((total - idle) as f64 * 100.0 / total as f64)
}
fn filetime(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

#[tauri::command]
pub fn local_resources(state: tauri::State<'_, ResourceState>) -> Resources {
    let mut idle = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let cpu_ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0;
    let cpu_percent = if cpu_ok {
        let current = CpuTimes {
            idle: filetime(idle),
            kernel: filetime(kernel),
            user: filetime(user),
        };
        state.0.lock().ok().and_then(|mut previous| {
            let result = previous.and_then(|old| cpu_usage(old, current));
            *previous = Some(current);
            result
        })
    } else {
        None
    };
    let mut memory = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    let memory = if unsafe { GlobalMemoryStatusEx(&mut memory) } != 0 && memory.ullTotalPhys > 0 {
        memory
            .ullTotalPhys
            .checked_sub(memory.ullAvailPhys)
            .map(|used| Capacity {
                used,
                total: memory.ullTotalPhys,
            })
    } else {
        None
    };
    let root = std::env::var_os("SystemRoot").unwrap_or_default();
    let disk = capacity_of(&root);
    let disk_path = std::path::Path::new(&root)
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_default();
    Resources {
        cpu_error: !cpu_ok,
        cpu_percent,
        memory,
        disk,
        disk_path,
        disks: fixed_disks(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drive_roots_follow_logical_drive_mask() {
        assert_eq!(drive_roots(0), Vec::<String>::new());
        assert_eq!(drive_roots(0b1100), vec!["C:\\", "D:\\"]);
        assert_eq!(drive_roots(1 << 25), vec!["Z:\\"]);
    }
    #[test]
    fn fixed_disks_include_system_drive() {
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let disks = fixed_disks();
        assert!(
            disks
                .iter()
                .any(|disk| disk.path.eq_ignore_ascii_case(&system))
        );
        assert!(
            disks
                .iter()
                .all(|disk| disk.used <= disk.total && disk.total > 0)
        );
    }
    #[test]
    fn cpu_usage_handles_deltas_and_invalid_samples() {
        let start = CpuTimes {
            idle: 100,
            kernel: 300,
            user: 100,
        };
        assert_eq!(
            cpu_usage(
                start,
                CpuTimes {
                    idle: 150,
                    kernel: 400,
                    user: 200
                }
            ),
            Some(75.0)
        );
        assert_eq!(cpu_usage(start, start), None);
        assert_eq!(
            cpu_usage(
                start,
                CpuTimes {
                    idle: 250,
                    kernel: 400,
                    user: 100
                }
            ),
            None
        );
        assert_eq!(
            cpu_usage(
                start,
                CpuTimes {
                    idle: 0,
                    kernel: 0,
                    user: 0
                }
            ),
            None
        );
    }
}
