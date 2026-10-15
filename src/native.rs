//! Small Windows boundary: tray, global shortcut, dark system frame and startup registration.
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path, sync::{OnceLock, mpsc::Sender}};
use crate::i18n;
use windows_sys::Win32::{Foundation::*, System::{LibraryLoader::GetModuleHandleW, Threading::*, Registry::*},
    UI::{WindowsAndMessaging::*, Shell::*, Input::KeyboardAndMouse::*}, Graphics::Dwm::*};

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
    for (target, value) in icon.szTip.iter_mut().take(127).zip(message.encode_utf16()) { *target = value; }
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
pub fn theme_frame(window: &slint::Window, owner: Option<HWND>, is_dark: bool) {
    let Some(hwnd) = hwnd(window) else { return; };
    unsafe {
        let dark: i32 = is_dark as i32; let color: u32 = if is_dark { 0x000a0a0a } else { 0x00f0f4f5 }; let border: u32 = if is_dark { 0x002b2b2b } else { 0x00d0d9dd };
        let text: u32 = if is_dark { 0x00ecf1f4 } else { 0x001e2529 }; let corner: i32 = 2;
        DwmSetWindowAttribute(hwnd, 20, (&dark as *const i32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 36, (&text as *const u32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 35, (&color as *const u32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 34, (&border as *const u32).cast(), 4);
        DwmSetWindowAttribute(hwnd, 33, (&corner as *const i32).cast(), 4);
        if let Some(owner) = owner { SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner as isize); }
    }
}
pub fn show(window: &slint::Window) {
    let _ = window.show();
    if let Some(hwnd) = hwnd(window) { unsafe { ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd); } }
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
