use std::process::Command;

use anyhow::Result;
use slint::winit_030::{winit, WinitWindowAccessor};
use winit::window::WindowAttributes;

pub const LOGIN_SCRIPT: &str = "claude auth login";

const POPOVER_TITLE: &str = "ClaudeTray popover";
const POPOVER_MARGIN: f32 = 12.0;

pub fn select_backend() -> Result<(), slint::PlatformError> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_window_attributes_hook(window_attributes)
        .select()
}

#[cfg(windows)]
fn window_attributes(attributes: WindowAttributes) -> WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;
    let popover = attributes.title == POPOVER_TITLE;
    let attributes = attributes.with_theme(Some(winit::window::Theme::Dark));
    if popover {
        attributes
            .with_skip_taskbar(true)
            .with_undecorated_shadow(true)
            .with_resizable(false)
    } else {
        attributes
    }
}

#[cfg(not(windows))]
fn window_attributes(attributes: WindowAttributes) -> WindowAttributes {
    if attributes.title == POPOVER_TITLE {
        attributes.with_resizable(false).with_transparent(true)
    } else {
        attributes
    }
}

#[cfg(windows)]
pub fn cursor() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut point = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut point) } != 0).then_some((point.x, point.y))
}

#[cfg(not(windows))]
pub fn cursor() -> Option<(i32, i32)> {
    None
}

#[cfg(windows)]
pub fn place_popover(window: &slint::Window, anchor: Option<(i32, i32)>, width: f32, height: f32) {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

    let Some((x, y)) = anchor else { return };
    let empty = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: empty,
        rcWork: empty,
        dwFlags: 0,
    };
    let (mut dpi_x, mut dpi_y) = (96, 96);
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return;
        }
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
    }
    let scale = dpi_x as f32 / 96.0;
    let (width, height) = ((width * scale) as i32, (height * scale) as i32);
    let margin = (POPOVER_MARGIN * scale) as i32;
    let (work, screen) = (info.rcWork, info.rcMonitor);

    let left = (x - width / 2).clamp(
        work.left + margin,
        (work.right - width - margin).max(work.left),
    );
    let top = if work.top > screen.top && y < work.top {
        work.top + margin
    } else {
        (work.bottom - height - margin).max(work.top)
    };
    window.set_position(slint::PhysicalPosition::new(left, top));
}

#[cfg(not(windows))]
pub fn place_popover(
    window: &slint::Window,
    _anchor: Option<(i32, i32)>,
    width: f32,
    _height: f32,
) {
    let position = window.with_winit_window(|winit_window| {
        let monitor = winit_window
            .current_monitor()
            .or_else(|| winit_window.primary_monitor())?;
        let scale = monitor.scale_factor() as f32;
        let margin = POPOVER_MARGIN * scale;
        let origin = monitor.position();
        let x = origin.x + monitor.size().width as i32 - (width * scale + margin) as i32;
        let y = origin.y + (36.0 * scale) as i32;
        Some(slint::PhysicalPosition::new(x, y))
    });
    if let Some(Some(position)) = position {
        window.set_position(position);
    }
}

pub fn focus_popover(window: &slint::Window) {
    window.with_winit_window(|winit_window| {
        #[cfg(windows)]
        round_corners(winit_window);
        winit_window.focus_window();
    });
}

pub fn is_focus_lost(event: &winit::event::WindowEvent) -> bool {
    matches!(event, winit::event::WindowEvent::Focused(false))
}

#[cfg(windows)]
fn round_corners(window: &winit::window::Window) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = window.window_handle() else {
        return;
    };
    if let RawWindowHandle::Win32(handle) = handle.as_raw() {
        let preference = DWMWCP_ROUND;
        unsafe {
            DwmSetWindowAttribute(
                handle.hwnd.get() as _,
                DWMWA_WINDOW_CORNER_PREFERENCE as _,
                &preference as *const _ as _,
                size_of_val(&preference) as u32,
            );
        }
    }
}

pub fn hotkey_needs_x11() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(windows)]
pub fn open_login_terminal() -> Result<()> {
    Command::new("cmd")
        .args(["/C", "start", "ClaudeTray", "cmd", "/K", LOGIN_SCRIPT])
        .spawn()?;
    Ok(())
}

#[cfg(not(windows))]
pub fn open_login_terminal() -> Result<()> {
    let script = format!("{LOGIN_SCRIPT}; echo; read -r _");
    let preferred = std::env::var("TERMINAL").ok();
    let candidates: Vec<(&str, Vec<&str>)> = preferred
        .as_deref()
        .map(|terminal| (terminal, vec!["-e", "sh", "-c"]))
        .into_iter()
        .chain([
            ("x-terminal-emulator", vec!["-e", "sh", "-c"]),
            ("gnome-terminal", vec!["--", "sh", "-c"]),
            ("konsole", vec!["-e", "sh", "-c"]),
            ("xfce4-terminal", vec!["-x", "sh", "-c"]),
            ("kitty", vec!["sh", "-c"]),
            ("alacritty", vec!["-e", "sh", "-c"]),
            ("xterm", vec!["-e", "sh", "-c"]),
        ])
        .collect();
    for (terminal, args) in candidates {
        if Command::new(terminal)
            .args(&args)
            .arg(&script)
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
    }
    anyhow::bail!("no terminal emulator found — run `{LOGIN_SCRIPT}` manually")
}

#[cfg(windows)]
pub fn tray_foreground() -> [u8; 3] {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let light = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_value::<u32, _>("SystemUsesLightTheme"))
        .unwrap_or(0)
        == 1;
    if light {
        [0, 0, 0]
    } else {
        [255, 255, 255]
    }
}

#[cfg(not(windows))]
pub fn tray_foreground() -> [u8; 3] {
    [235, 235, 235]
}

#[cfg(windows)]
pub fn attach_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
pub fn attach_console() {}

pub fn notify(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .appname("ClaudeTray")
        .summary(title)
        .body(body)
        .show();
}
