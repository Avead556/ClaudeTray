use std::path::PathBuf;

fn directory() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join("ClaudeTray")
}

pub fn ensure_directory() {
    let _ = std::fs::create_dir_all(directory());
}

pub fn accounts() -> PathBuf {
    directory().join("accounts.json")
}

pub fn usage() -> PathBuf {
    directory().join("usage.json")
}

pub fn settings() -> PathBuf {
    directory().join("settings.json")
}
