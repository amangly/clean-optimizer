use crate::error::Result;
use crate::types::LiveMetrics;

pub fn snapshot() -> Result<LiveMetrics> {
    let gpu = gpu_nvidia();
    Ok(LiveMetrics {
        cpu_pct: cpu_pct(),
        gpu_pct: gpu.as_ref().map(|g| g.util),
        ram_pct: ram_pct(),
        cpu_temp: None,
        gpu_temp: gpu.as_ref().and_then(|g| g.temp),
        fps: None,
        fps_1pct: None,
        note: if gpu.is_some() {
            "CPU, RAM, and NVIDIA GPU from Windows / nvidia-smi. FPS needs PresentMon.".into()
        } else {
            "CPU and RAM from Windows. GPU via nvidia-smi when it is installed.".into()
        },
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

struct GpuSnap {
    util: u32,
    temp: Option<f64>,
}

fn gpu_nvidia() -> Option<GpuSnap> {
    let smi = [
        r"C:\Windows\System32\nvidia-smi.exe",
        r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
    ]
    .into_iter()
    .find(|p| std::path::Path::new(p).exists())?;
    let out = hidden(&smi)
        .args([
            "--query-gpu=utilization.gpu,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    parse_nvidia_smi(&String::from_utf8_lossy(&out.stdout))
}

fn parse_nvidia_smi(text: &str) -> Option<GpuSnap> {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    let mut parts = line.split(',').map(|s| s.trim());
    let util = parts.next()?.parse::<u32>().ok()?;
    let temp = parts.next().and_then(|s| s.parse::<f64>().ok());
    Some(GpuSnap { util, temp })
}

fn hidden(bin: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        crate::win::hidden_command(bin)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(bin)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_smi_line() {
        let snap = parse_nvidia_smi("12, 54\n").unwrap();
        assert_eq!(snap.util, 12);
        assert_eq!(snap.temp, Some(54.0));
    }
}
