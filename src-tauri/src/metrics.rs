use crate::error::Result;
use crate::types::LiveMetrics;

pub fn snapshot() -> Result<LiveMetrics> {
    Ok(LiveMetrics {
        cpu_pct: cpu_pct(),
        gpu_pct: None,
        ram_pct: ram_pct(),
        cpu_temp: None,
        gpu_temp: None,
        fps: None,
        fps_1pct: None,
        note: "CPU and RAM come from Windows. GPU temp, power, and FPS need PresentMon or LibreHardwareMonitor in resources.".into(),
    })
}

pub fn total_ram_gb() -> Option<f64> {
    #[cfg(windows)]
    {
        memory_status().map(|m| m.total_phys as f64 / 1024.0 / 1024.0 / 1024.0)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn ram_pct() -> Option<u32> {
    #[cfg(windows)]
    {
        memory_status().map(|m| m.memory_load)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn cpu_pct() -> Option<u32> {
    #[cfg(windows)]
    {
        crate::win_cpu::sample()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
struct Mem {
    memory_load: u32,
    total_phys: u64,
}

#[cfg(windows)]
fn memory_status() -> Option<Mem> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    unsafe {
        let mut mem = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
            ullAvailExtendedVirtual: 0,
        };
        if GlobalMemoryStatusEx(&mut mem) == 0 {
            return None;
        }
        Some(Mem {
            memory_load: mem.dwMemoryLoad,
            total_phys: mem.ullTotalPhys,
        })
    }
}
