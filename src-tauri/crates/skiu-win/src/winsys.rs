use skiu_core::{Fg, Sys, WindowInfo};
use windows_sys::Win32::{
    Foundation::{CloseHandle, BOOL, HWND, LPARAM},
    Graphics::{
        Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
        Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITORINFOEXW,
            MONITOR_DEFAULTTONEAREST,
        },
    },
    System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    },
};

pub struct WinSys;

/// Programmes systeme ou de l'application elle-meme, inutiles dans la liste des jeux.
const IGNORED: &[&str] = &[
    "explorer.exe",
    "applicationframehost.exe",
    "textinputhost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "systemsettings.exe",
    "lockapp.exe",
    "taskmgr.exe",
    "nvcplui.exe",
    "nvidia app.exe",
    "msedgewebview2.exe",
    "skiu-vibrance.exe",
    "skiu vibrance.exe",
];

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

unsafe fn process_path(pid: u32) -> Option<String> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
    CloseHandle(handle);
    if ok == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

unsafe fn window_title(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

unsafe fn monitor_name(hwnd: HWND) -> Option<String> {
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if monitor.is_null() {
        return None;
    }
    let mut info: MONITORINFOEXW = core::mem::zeroed();
    info.monitorInfo.cbSize = core::mem::size_of::<MONITORINFOEXW>() as u32;
    let ptr = &mut info as *mut MONITORINFOEXW as *mut MONITORINFO;
    if GetMonitorInfoW(monitor, ptr) == 0 {
        return None;
    }
    Some(wide_to_string(&info.szDevice))
}

unsafe fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let hr = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        &mut cloaked as *mut u32 as *mut core::ffi::c_void,
        core::mem::size_of::<u32>() as u32,
    );
    hr == 0 && cloaked != 0
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam as *mut Vec<usize>);
    list.push(hwnd as usize);
    1
}

impl Sys for WinSys {
    fn foreground(&mut self) -> Option<Fg> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 {
                return None;
            }
            let path = process_path(pid)?;
            Some(Fg {
                exe: file_name(&path),
                title: window_title(hwnd),
                monitor: monitor_name(hwnd),
                path: Some(path),
            })
        }
    }

    fn windows(&mut self) -> Vec<WindowInfo> {
        let mut handles: Vec<usize> = Vec::new();
        unsafe {
            EnumWindows(Some(collect), &mut handles as *mut Vec<usize> as LPARAM);
        }
        let mut out: Vec<WindowInfo> = Vec::new();
        for h in handles {
            let hwnd = h as HWND;
            unsafe {
                if IsWindowVisible(hwnd) == 0 || is_cloaked(hwnd) {
                    continue;
                }
                let title = window_title(hwnd);
                if title.is_empty() {
                    continue;
                }
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, &mut pid);
                let Some(path) = process_path(pid) else {
                    continue;
                };
                let exe = file_name(&path);
                let lower = exe.to_lowercase();
                if IGNORED.contains(&lower.as_str())
                    || out.iter().any(|w| w.exe.eq_ignore_ascii_case(&exe))
                {
                    continue;
                }
                out.push(WindowInfo {
                    exe,
                    title,
                    path: Some(path),
                });
            }
        }
        out.sort_by_key(|w| w.exe.to_lowercase());
        out
    }
}
