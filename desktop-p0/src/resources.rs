use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        ProcessStatus::{
            K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
        },
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::{
            GetProcessTimes, GetSystemTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

#[derive(Default)]
pub struct ResourceState {
    system: Mutex<Option<CpuTimes>>,
    app: Mutex<Option<AppSample>>,
}
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
/// AgentMeter's own process tree: the app, its WebView2 processes and running collectors.
#[derive(Serialize)]
pub struct AppUsage {
    /// Share of the whole machine, like Task Manager; unknown until the second sample.
    cpu_percent: Option<f64>,
    memory: u64,
    processes: usize,
}
/// CPU time per process, keyed by pid and creation time so a reused pid starts over.
struct AppSample {
    system_total: u64,
    cpu: HashMap<(u32, u64), u64>,
}
struct ProcessSample {
    created: u64,
    cpu: u64,
    memory: u64,
}
#[derive(Serialize)]
pub struct Resources {
    app: Option<AppUsage>,
    cpu_error: bool,
    cpu_percent: Option<f64>,
    memory: Option<Capacity>,
    disk: Option<Capacity>,
    disk_path: String,
    disks: Vec<Disk>,
}

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: constructed only from a valid handle this struct owns; closed exactly once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// (pid, parent pid) of every running process.
fn process_parents() -> Option<Vec<(u32, u32)>> {
    // SAFETY: the entry is zeroed with its size set, and the snapshot handle is checked and owned.
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle == INVALID_HANDLE_VALUE {
            return None;
        }
        let snapshot = OwnedHandle(handle);
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut parents = Vec::new();
        let mut more = Process32FirstW(snapshot.0, &mut entry) != 0;
        while more {
            parents.push((entry.th32ProcessID, entry.th32ParentProcessID));
            more = Process32NextW(snapshot.0, &mut entry) != 0;
        }
        Some(parents)
    }
}

fn sample_process(pid: u32) -> Option<ProcessSample> {
    // SAFETY: the handle is checked and owned; every out-structure is initialized and sized.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let process = OwnedHandle(handle);
        let (mut created, mut exited, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        if GetProcessTimes(process.0, &mut created, &mut exited, &mut kernel, &mut user) == 0 {
            return None;
        }
        // The private working set avoids counting memory WebView2 processes share. Windows
        // versions without it fall back to the full working set.
        let mut extended: PROCESS_MEMORY_COUNTERS_EX2 = std::mem::zeroed();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
        extended.cb = size;
        let memory = if K32GetProcessMemoryInfo(process.0, (&raw mut extended).cast(), size) != 0 {
            extended.PrivateWorkingSetSize
        } else {
            let mut basic: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            basic.cb = size;
            if K32GetProcessMemoryInfo(process.0, &mut basic, size) == 0 {
                return None;
            }
            basic.WorkingSetSize
        };
        Some(ProcessSample {
            created: filetime(created),
            cpu: filetime(kernel) + filetime(user),
            memory: memory as u64,
        })
    }
}

/// The root and its descendants. A child must not be older than its parent: Windows keeps the
/// parent pid after the parent exits, so a reused pid would otherwise adopt strangers.
fn process_tree(
    root: u32,
    parents: &[(u32, u32)],
    sample: impl Fn(u32) -> Option<ProcessSample>,
) -> HashMap<u32, ProcessSample> {
    let mut tree = HashMap::new();
    let Some(first) = sample(root) else {
        return tree;
    };
    let mut pending = vec![(root, first.created)];
    tree.insert(root, first);
    while let Some((parent, created)) = pending.pop() {
        for &(pid, _) in parents.iter().filter(|(_, owner)| *owner == parent) {
            if tree.contains_key(&pid) {
                continue;
            }
            if let Some(child) = sample(pid).filter(|child| child.created >= created) {
                pending.push((pid, child.created));
                tree.insert(pid, child);
            }
        }
    }
    tree
}

fn app_cpu_percent(previous: &AppSample, current: &AppSample) -> Option<f64> {
    let total = current.system_total.checked_sub(previous.system_total)?;
    if total == 0 {
        return None;
    }
    // A process that appeared since the last sample spent all its time within the interval.
    let used: u64 = current
        .cpu
        .iter()
        .map(|(key, cpu)| cpu.saturating_sub(previous.cpu.get(key).copied().unwrap_or(0)))
        .sum();
    Some((used as f64 * 100.0 / total as f64).min(100.0))
}

fn app_usage(state: &Mutex<Option<AppSample>>, system: Option<CpuTimes>) -> Option<AppUsage> {
    let tree = process_tree(std::process::id(), &process_parents()?, sample_process);
    if tree.is_empty() {
        return None;
    }
    let cpu_percent = system.and_then(|system| {
        let current = AppSample {
            // Kernel time already includes idle time, so this is all CPU time on all cores.
            system_total: system.kernel.checked_add(system.user)?,
            cpu: tree
                .iter()
                .map(|(pid, process)| ((*pid, process.created), process.cpu))
                .collect(),
        };
        let mut previous = state.lock().ok()?;
        let percent = previous
            .as_ref()
            .and_then(|old| app_cpu_percent(old, &current));
        *previous = Some(current);
        percent
    });
    Some(AppUsage {
        cpu_percent,
        memory: tree.values().map(|process| process.memory).sum(),
        processes: tree.len(),
    })
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
    let current = cpu_ok.then(|| CpuTimes {
        idle: filetime(idle),
        kernel: filetime(kernel),
        user: filetime(user),
    });
    let cpu_percent = current.and_then(|current| {
        let mut previous = state.system.lock().ok()?;
        let result = previous.and_then(|old| cpu_usage(old, current));
        *previous = Some(current);
        result
    });
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
        app: app_usage(&state.app, current),
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
    fn process(created: u64, cpu: u64) -> ProcessSample {
        ProcessSample {
            created,
            cpu,
            memory: 10,
        }
    }
    #[test]
    fn process_tree_follows_descendants_and_rejects_reused_parent_pids() {
        // 5 → 6 → 7 belong to the app; 8 claims parent 6 but is older, so its real parent is gone.
        let parents = [(5, 1), (6, 5), (7, 6), (8, 6), (9, 1), (5, 5)];
        let tree = process_tree(5, &parents, |pid| match pid {
            5 => Some(process(100, 1)),
            6 => Some(process(110, 1)),
            7 => Some(process(120, 1)),
            8 => Some(process(50, 1)),
            _ => Some(process(200, 1)),
        });
        let mut pids: Vec<_> = tree.into_keys().collect();
        pids.sort_unstable();
        assert_eq!(pids, vec![5, 6, 7]);
        assert!(process_tree(5, &parents, |_| None).is_empty());
    }
    #[test]
    fn app_cpu_is_a_share_of_the_machine_across_changing_processes() {
        let sample = |system_total, cpu: &[((u32, u64), u64)]| AppSample {
            system_total,
            cpu: cpu.iter().copied().collect(),
        };
        let before = sample(1_000, &[((1, 1), 50), ((2, 1), 40)]);
        // pid 2 exited and its pid was reused by a new child that spent 30.
        let after = sample(2_000, &[((1, 1), 70), ((2, 9), 30)]);
        assert_eq!(app_cpu_percent(&before, &after), Some(5.0));
        assert_eq!(app_cpu_percent(&before, &before), None);
        assert_eq!(app_cpu_percent(&after, &before), None);
    }
    #[test]
    fn app_usage_measures_this_process() {
        let state = Mutex::new(None);
        let first = app_usage(
            &state,
            Some(CpuTimes {
                idle: 0,
                kernel: 1,
                user: 1,
            }),
        )
        .unwrap();
        assert!(first.cpu_percent.is_none());
        assert!(first.processes >= 1);
        assert!(first.memory > 100_000, "{} bytes", first.memory);
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
