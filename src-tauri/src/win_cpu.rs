#![cfg(windows)]

use windows_sys::Win32::System::SystemInformation::GetLogicalProcessorInformationEx;
use windows_sys::Win32::System::Threading::GetSystemTimes;

pub fn sample() -> Option<u32> {
    let (idle1, kernel1, user1) = times()?;
    std::thread::sleep(std::time::Duration::from_millis(80));
    let (idle2, kernel2, user2) = times()?;
    let idle = idle2.saturating_sub(idle1);
    let kernel = kernel2.saturating_sub(kernel1);
    let user = user2.saturating_sub(user1);
    let total = kernel.saturating_add(user);
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle);
    Some(((busy * 100) / total) as u32)
}

pub fn processor_cores() -> Option<Vec<(u8, u64)>> {
    unsafe {
        let mut len = 0u32;
        let _ = GetLogicalProcessorInformationEx(0, std::ptr::null_mut(), &mut len);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        if GetLogicalProcessorInformationEx(0, buf.as_mut_ptr().cast(), &mut len) == 0 {
            return None;
        }
        let mut cores = Vec::new();
        let mut pos = 0usize;
        let end = len as usize;
        while pos + 8 <= end {
            let relationship = u32::from_le_bytes(buf[pos..pos + 4].try_into().ok()?);
            let size = u32::from_le_bytes(buf[pos + 4..pos + 8].try_into().ok()?) as usize;
            if size == 0 || pos + size > end {
                return None;
            }
            if relationship == 0 && pos + 42 <= buf.len() {
                let class = buf[pos + 9];
                let mask = u64::from_le_bytes(buf[pos + 32..pos + 40].try_into().ok()?);
                let group = u16::from_le_bytes(buf[pos + 40..pos + 42].try_into().ok()?);
                if group == 0 {
                    cores.push((class, mask));
                }
            }
            pos += size;
        }
        Some(cores)
    }
}

fn times() -> Option<(u64, u64, u64)> {
    unsafe {
        let mut idle = 0u64;
        let mut kernel = 0u64;
        let mut user = 0u64;
        if GetSystemTimes(
            &mut idle as *mut u64 as *mut _,
            &mut kernel as *mut u64 as *mut _,
            &mut user as *mut u64 as *mut _,
        ) == 0
        {
            return None;
        }
        Some((idle, kernel, user))
    }
}
