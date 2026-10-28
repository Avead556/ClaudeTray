use std::process::Command;

use anyhow::Result;

pub const LOGIN_SCRIPT: &str = "claude auth login";

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
