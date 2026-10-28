use std::fs;

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub hotkey_enabled: bool,
    pub hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey_enabled: true,
            hotkey: "ctrl+alt+shift+KeyA".into(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        fs::read(paths::settings())
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        paths::ensure_directory();
        if let Ok(data) = serde_json::to_vec_pretty(self) {
            let _ = fs::write(paths::settings(), data);
        }
    }

    pub fn hotkey_title(&self) -> String {
        self.hotkey
            .split('+')
            .map(|part| match part.to_lowercase().as_str() {
                "ctrl" | "control" => "Ctrl".to_owned(),
                "alt" | "option" => "Alt".to_owned(),
                "shift" => "Shift".to_owned(),
                "super" | "meta" | "cmd" | "win" => "Win".to_owned(),
                _ => part
                    .trim_start_matches("Key")
                    .trim_start_matches("Digit")
                    .to_owned(),
            })
            .collect::<Vec<_>>()
            .join("+")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hotkey_title() {
        assert_eq!(Settings::default().hotkey_title(), "Ctrl+Alt+Shift+A");
    }
}
