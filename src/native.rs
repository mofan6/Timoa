//! Small Windows boundary: tray, global shortcut, dark system frame and startup registration.
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path, sync::{OnceLock, mpsc::Sender}};
use crate::i18n;
use windows_sys::Win32::{Foundation::*, System::{LibraryLoader::{GetModuleHandleW, GetProcAddress}, Threading::*, Registry::*},
    UI::{WindowsAndMessaging::*, Shell::*, Input::KeyboardAndMouse::*, Controls::MARGINS}, Graphics::{Dwm::*, Gdi::*}};

pub enum Event { Show, Quit }
static EVENTS: OnceLock<Sender<Event>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
const CLASS: &str = "TIMOMessageWindow";
const TRAY_MESSAGE: u32 = WM_APP + 31;
const SHOW_MESSAGE: u32 = WM_APP + 32;
pub fn wide(s: impl AsRef<OsStr>) -> Vec<u16> { s.as_ref().encode_wide().chain(Some(0)).collect() }
pub fn error(message: &str) {
    unsafe { MessageBoxW(std::ptr::null_mut(), wide(i18n::message(message)).as_ptr(), wide("Timoa").as_ptr(), MB_OK | MB_ICONERROR); }
}

pub struct Instance(HANDLE);
impl Instance {
    pub fn claim(show_existing: bool) -> Result<Option<Self>, String> {
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, wide("Local\\TIMO.Native.SingleInstance.v1").as_ptr()) };
        if mutex.is_null() { return Err(i18n::t("无法建立程序实例锁。").into()); }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            let hwnd = unsafe { FindWindowW(wide(CLASS).as_ptr(), std::ptr::null()) };
            if show_existing && !hwnd.is_null() { unsafe { PostMessageW(hwnd, SHOW_MESSAGE, 0, 0); } }
            unsafe { CloseHandle(mutex); }
            return Ok(None);
        }
        Ok(Some(Self(mutex)))
    }
}
impl Drop for Instance { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }

pub struct Bridge { hwnd: HWND, pub hotkey: bool }
impl Bridge {
    pub fn new(sender: Sender<Event>) -> Result<Self, String> {
        let _ = EVENTS.set(sender);
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = wide(CLASS);
            let mut class: WNDCLASSW = std::mem::zeroed();
            class.lpfnWndProc = Some(window_proc);
            class.hInstance = instance;
            class.lpszClassName = class_name.as_ptr();
            RegisterClassW(&class);
            // A hidden top-level tool window also receives Explorer's TaskbarCreated broadcast.
            let _ = TASKBAR_CREATED.set(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()));
            let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, class_name.as_ptr(), class_name.as_ptr(), WS_POPUP,
                0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), instance, std::ptr::null());
            if hwnd.is_null() { return Err(i18n::t("无法建立托盘消息窗口。").into()); }
            add_tray(hwnd);
            let hotkey = RegisterHotKey(hwnd, 0x544E, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, 0x54) != 0;
            Ok(Self { hwnd, hotkey })
        }
    }
    pub fn retry_hotkey(&mut self) {
        if !self.hotkey {
            self.hotkey = unsafe { RegisterHotKey(self.hwnd, 0x544E, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, 0x54) != 0 };
        }
    }
    pub fn tooltip(&self, message: &str) {
        unsafe {
            let mut icon: NOTIFYICONDATAW = std::mem::zeroed();
            icon.cbSize = std::mem::size_of_val(&icon) as u32;
            icon.hWnd = self.hwnd; icon.uID = 1; icon.uFlags = NIF_TIP;
            put_tip(&mut icon, message); Shell_NotifyIconW(NIM_MODIFY, &icon);
        }
    }
}
unsafe fn add_tray(hwnd: HWND) {
    unsafe {
        let mut icon: NOTIFYICONDATAW = std::mem::zeroed();
        icon.cbSize = std::mem::size_of_val(&icon) as u32;
        icon.hWnd = hwnd; icon.uID = 1; icon.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        icon.uCallbackMessage = TRAY_MESSAGE;
        icon.hIcon = LoadImageW(GetModuleHandleW(std::ptr::null()), 1usize as _, IMAGE_ICON, 32, 32, LR_SHARED) as _;
        if icon.hIcon.is_null() { icon.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION); }
        put_tip(&mut icon, i18n::t("Timoa · 专注此刻"));
        Shell_NotifyIconW(NIM_ADD, &icon);
    }
}
fn put_tip(icon: &mut NOTIFYICONDATAW, message: &str) {
    // NOTIFYICONDATAW is packed on x86. Build the text in an aligned local array.
    let mut tip = [0u16; 128];
    for (target, value) in tip.iter_mut().take(127).zip(message.encode_utf16()) { *target = value; }
    icon.szTip = tip;
}
impl Drop for Bridge {
    fn drop(&mut self) { unsafe {
        UnregisterHotKey(self.hwnd, 0x544E);
        let mut icon: NOTIFYICONDATAW = std::mem::zeroed();
        icon.cbSize = std::mem::size_of_val(&icon) as u32; icon.hWnd = self.hwnd; icon.uID = 1;
        Shell_NotifyIconW(NIM_DELETE, &icon); DestroyWindow(self.hwnd);
    } }
}
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message != 0 && TASKBAR_CREATED.get().is_some_and(|id| *id == message) { add_tray(hwnd); return 0; }
        if message == SHOW_MESSAGE || message == WM_HOTKEY {
            if let Some(sender) = EVENTS.get() { let _ = sender.send(Event::Show); }
            return 0;
        }
        if message == TRAY_MESSAGE {
            let event = l as u32 & 0xffff;
            if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK || event == 0x400 {
                if let Some(sender) = EVENTS.get() { let _ = sender.send(Event::Show); }
            } else if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                let menu = CreatePopupMenu();
                AppendMenuW(menu, MF_STRING, 1, wide(i18n::t("打开 Timoa")).as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(menu, MF_STRING, 2, wide(i18n::t("退出")).as_ptr());
                let mut point: POINT = std::mem::zeroed(); GetCursorPos(&mut point);
                SetForegroundWindow(hwnd);
                let selection = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                    point.x, point.y, 0, hwnd, std::ptr::null());
                DestroyMenu(menu);
                if let Some(sender) = EVENTS.get() {
                    if selection == 1 { let _ = sender.send(Event::Show); }
                    if selection == 2 { let _ = sender.send(Event::Quit); }
                }
                PostMessageW(hwnd, WM_NULL, 0, 0);
            }
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}

pub fn hwnd(window: &slint::Window) -> Option<HWND> {
    use raw_window_handle::HasWindowHandle;
    let provider = window.window_handle();
    match provider.window_handle().ok()?.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as HWND), _ => None,
    }
}
pub struct FramePalette {
    pub dark: bool,
    pub caption: slint::Color,
    pub text: slint::Color,
    pub border: slint::Color,
}
fn color_ref(color: slint::Color) -> u32 {
    // Windows COLORREF is 0x00BBGGRR, unlike Slint's ARGB representation.
    color.red() as u32 | (color.green() as u32) << 8 | (color.blue() as u32) << 16
}
pub fn theme_frame(window: &slint::Window, owner: Option<HWND>, palette: FramePalette) {
    let Some(hwnd) = hwnd(window) else { return; };
    unsafe {
        let dark: i32 = palette.dark as i32;
        let color = color_ref(palette.caption);
        let border = color_ref(palette.border);
        let text = color_ref(palette.text);
        let corner: i32 = 2;
        // Prefer the current dark-frame attribute. Older Windows 10 builds
        // expose attribute 19 instead; unsupported attributes are harmless on 7.
        if DwmSetWindowAttribute(hwnd, 20, (&dark as *const i32).cast(), 4) < 0 {
            DwmSetWindowAttribute(hwnd, 19, (&dark as *const i32).cast(), 4);
        }
        DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 35, (&color as *const u32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 33, (&corner as *const i32).cast(), 4);
        if let Some(owner) = owner { SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner as _); }
    }
}
pub fn show(window: &slint::Window) {
    let _ = window.show();
    if let Some(hwnd) = hwnd(window) { unsafe { ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd); } }
}

const FRAME_PROC: &str = "Timoa.OriginalWindowProc";
const MINI_FRAME: &str = "Timoa.MiniWindow";

// Keep a native resizable frame and DWM shadow/corners, while the client draws
// the caption. Each window stores its own previous procedure in a Win32 property.
pub fn configure_chrome(window: &slint::Window, mini: bool) {
    let Some(hwnd) = hwnd(window) else { return; };
    unsafe {
        if !GetPropW(hwnd, wide(FRAME_PROC).as_ptr()).is_null() { return; }
        let original = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        if original == 0 || SetPropW(hwnd, wide(FRAME_PROC).as_ptr(), original as HANDLE) == 0 { return; }
        if mini { SetPropW(hwnd, wide(MINI_FRAME).as_ptr(), 1usize as HANDLE); }
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, frame_proc as *const () as _);
        let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU;
        if mini {
            style &= !(WS_MINIMIZEBOX | WS_MAXIMIZEBOX);
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ((ex_style | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW) as _);
        } else { style |= WS_MINIMIZEBOX | WS_MAXIMIZEBOX; }
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as _);
        let margins = MARGINS { cxLeftWidth: 1, cxRightWidth: 1, cyTopHeight: 1, cyBottomHeight: 1 };
        DwmExtendFrameIntoClientArea(hwnd, &margins);
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

unsafe extern "system" fn frame_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let original = GetPropW(hwnd, wide(FRAME_PROC).as_ptr());
        if original.is_null() { return DefWindowProcW(hwnd, message, w, l); }
        let original: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT = std::mem::transmute(original);
        if message == WM_NCCALCSIZE && w != 0 {
            if IsZoomed(hwnd) != 0 {
                let mut info: MONITORINFO = std::mem::zeroed(); info.cbSize = std::mem::size_of_val(&info) as u32;
                if GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) != 0 {
                    (*(l as *mut NCCALCSIZE_PARAMS)).rgrc[0] = info.rcWork;
                }
            }
            return 0;
        }
        // Both windows retain native edge/corner resize handling. Keeping this
        // in non-client hit testing avoids Slint pointer capture during resizing.
        if message == WM_NCHITTEST && IsZoomed(hwnd) == 0 {
            let mut rect: RECT = std::mem::zeroed(); GetWindowRect(hwnd, &mut rect);
            let x = (l as u32 & 0xffff) as i16 as i32;
            let y = ((l as u32 >> 16) & 0xffff) as i16 as i32;
            let dpi = window_dpi(hwnd);
            let border = system_metric_for_dpi(SM_CXSIZEFRAME, dpi) + system_metric_for_dpi(SM_CXPADDEDBORDER, dpi);
            let left = x < rect.left + border; let right = x >= rect.right - border;
            let top = y < rect.top + border; let bottom = y >= rect.bottom - border;
            let hit = match (left, right, top, bottom) {
                (true, _, true, _) => HTTOPLEFT, (_, true, true, _) => HTTOPRIGHT,
                (true, _, _, true) => HTBOTTOMLEFT, (_, true, _, true) => HTBOTTOMRIGHT,
                (true, _, _, _) => HTLEFT, (_, true, _, _) => HTRIGHT,
                (_, _, true, _) => HTTOP, (_, _, _, true) => HTBOTTOM, _ => HTCLIENT,
            };
            if hit != HTCLIENT { return hit as LRESULT; }
        }
        if message == WM_NCHITTEST { return HTCLIENT as LRESULT; }
        if message == WM_NCDESTROY {
            RemovePropW(hwnd, wide(FRAME_PROC).as_ptr()); RemovePropW(hwnd, wide(MINI_FRAME).as_ptr());
        }
        CallWindowProcW(Some(original), hwnd, message, w, l)
    }
}

pub fn caption_action(window: &slint::Window, action: &str) {
    let Some(hwnd) = hwnd(window) else { return; };
    unsafe {
        match action {
            "minimize" => { ShowWindow(hwnd, SW_MINIMIZE); }
            "maximize" => { ShowWindow(hwnd, if IsZoomed(hwnd) != 0 { SW_RESTORE } else { SW_MAXIMIZE }); }
            _ => {}
        }
    }
}
pub fn maximized(window: &slint::Window) -> bool { hwnd(window).is_some_and(|hwnd| unsafe { IsZoomed(hwnd) != 0 }) }

type DpiForWindow = unsafe extern "system" fn(HWND) -> u32;
type MetricForDpi = unsafe extern "system" fn(i32, u32) -> i32;
struct DpiApis { window: Option<DpiForWindow>, metric: Option<MetricForDpi> }
fn dpi_apis() -> &'static DpiApis {
    static APIS: OnceLock<DpiApis> = OnceLock::new();
    APIS.get_or_init(|| unsafe {
        let module = GetModuleHandleW(wide("user32.dll").as_ptr());
        DpiApis {
            window: GetProcAddress(module, c"GetDpiForWindow".as_ptr().cast())
                .map(|function| std::mem::transmute::<unsafe extern "system" fn() -> isize, DpiForWindow>(function)),
            metric: GetProcAddress(module, c"GetSystemMetricsForDpi".as_ptr().cast())
                .map(|function| std::mem::transmute::<unsafe extern "system" fn() -> isize, MetricForDpi>(function)),
        }
    })
}
fn window_dpi(hwnd: HWND) -> u32 {
    unsafe {
        if let Some(function) = dpi_apis().window { return function(hwnd).max(96); }
        let dc = GetDC(hwnd);
        if dc.is_null() { return 96; }
        let dpi = GetDeviceCaps(dc, LOGPIXELSX as i32).max(96) as u32;
        ReleaseDC(hwnd, dc);
        dpi
    }
}
fn system_metric_for_dpi(metric: i32, dpi: u32) -> i32 {
    unsafe {
        if let Some(function) = dpi_apis().metric { function(metric, dpi) }
        else { GetSystemMetrics(metric) }
    }
}

pub fn pointer_position(window: &slint::Window) -> Option<slint::LogicalPosition> {
    let hwnd = hwnd(window)?;
    unsafe {
        let mut point: POINT = std::mem::zeroed();
        if GetCursorPos(&mut point) == 0 || ScreenToClient(hwnd, &mut point) == 0 { return None; }
        let scale = window.scale_factor().max(0.1);
        Some(slint::LogicalPosition::new(point.x as f32 / scale, point.y as f32 / scale))
    }
}

pub fn place_mini(window: &slint::Window, parent: &slint::Window) {
    let Some(parent_hwnd) = hwnd(parent) else { return; };
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed(); info.cbSize = std::mem::size_of_val(&info) as u32;
        if GetMonitorInfoW(MonitorFromWindow(parent_hwnd, MONITOR_DEFAULTTONEAREST), &mut info) == 0 { return; }
        let scale = window.scale_factor(); let size = window.size(); let pos = parent.position();
        let x = (pos.x + parent.size().width as i32 - size.width as i32 - (24. * scale) as i32)
            .clamp(info.rcWork.left, (info.rcWork.right - size.width as i32).max(info.rcWork.left));
        let y = (pos.y + (64. * scale) as i32).clamp(info.rcWork.top, (info.rcWork.bottom - size.height as i32).max(info.rcWork.top));
        window.set_position(slint::PhysicalPosition::new(x, y));
    }
}

pub fn startup(enabled: bool) -> Result<(), String> {
    let executable = if enabled { Some(std::env::current_exe().map_err(|e| e.to_string())?) } else { None };
    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let result = RegCreateKeyExW(HKEY_CURRENT_USER, wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run").as_ptr(),
            0, std::ptr::null(), 0, KEY_SET_VALUE, std::ptr::null(), &mut key, std::ptr::null_mut());
        if result != 0 { return Err(i18n::t("无法修改开机启动设置。").into()); }
        let name = wide("TIMO");
        let result = if enabled {
            let exe = executable.as_ref().unwrap();
            let command = wide(format!("\"{}\" --startup", exe.display()));
            RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, command.as_ptr().cast(), (command.len() * 2) as u32)
        } else { RegDeleteValueW(key, name.as_ptr()) };
        RegCloseKey(key);
        if enabled && result == 0 {
            // Enabling also clears this program's previous Task Manager disable flag.
            let approval = RegDeleteKeyValueW(HKEY_CURRENT_USER,
                wide("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run").as_ptr(), name.as_ptr());
            if approval != 0 && approval != ERROR_FILE_NOT_FOUND && approval != ERROR_PATH_NOT_FOUND {
                return Err(i18n::t("无法更新 Windows 的启动许可，请在任务管理器中启用 Timoa。").into());
            }
        }
        if result == 0 || (!enabled && result == ERROR_FILE_NOT_FOUND) { Ok(()) }
        else { Err(i18n::t("未能保存开机启动设置。").into()) }
    }
}
pub fn startup_enabled() -> bool {
    let Ok(exe) = std::env::current_exe() else { return false; };
    let expected = format!("\"{}\" --startup", exe.display());
    unsafe {
        let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run"); let name = wide("TIMO");
        let mut size = 0u32;
        if RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ,
            std::ptr::null_mut(), std::ptr::null_mut(), &mut size) != 0 || !(2..=65536).contains(&size) { return false; }
        let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
        if RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ,
            std::ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size) != 0 { return false; }
        let length = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        let Ok(command) = String::from_utf16(&buffer[..length]) else { return false; };
        if !command.trim().eq_ignore_ascii_case(&expected) || !exe.is_file() { return false; }
        let mut approval = [0u8; 12]; let mut approval_size = approval.len() as u32;
        let result = RegGetValueW(HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run").as_ptr(),
            name.as_ptr(), RRF_RT_REG_BINARY, std::ptr::null_mut(), approval.as_mut_ptr().cast(), &mut approval_size);
        result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND || result == 0 && approval_size > 0 && approval[0] & 1 == 0
    }
}
pub fn open(path: &Path) {
    unsafe { ShellExecuteW(std::ptr::null_mut(), wide("open").as_ptr(), wide(path).as_ptr(),
        std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL); }
}
