use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};

const OAUTH_KEY: &str = "claudeAiOauth";
const ACCOUNT_KEY: &str = "oauthAccount";

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

fn custom_dir() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from)
}

fn credentials_path() -> PathBuf {
    custom_dir()
        .unwrap_or_else(|| home().join(".claude"))
        .join(".credentials.json")
}

fn config_path() -> PathBuf {
    custom_dir().unwrap_or_else(home).join(".claude.json")
}

fn read_object(path: &Path) -> Option<Map<String, Value>> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text).ok()? {
        Value::Object(object) => Some(object),
        _ => None,
    }
}

pub fn read_oauth() -> Option<Value> {
    read_object(&credentials_path())?.remove(OAUTH_KEY)
}

pub fn write_oauth(oauth: &Value) -> Result<()> {
    let path = credentials_path();
    let mut credentials = read_object(&path).unwrap_or_default();
    credentials.insert(OAUTH_KEY.into(), oauth.clone());
    write_private(&path, &serde_json::to_vec(&credentials)?)
}

pub fn read_account() -> Option<Value> {
    read_object(&config_path())?.remove(ACCOUNT_KEY)
}

pub fn write_account(account: &Value) -> Result<()> {
    let path = config_path();
    let Some(mut config) = read_object(&path) else {
        bail!("{} is not a JSON object", path.display());
    };
    config.insert(ACCOUNT_KEY.into(), account.clone());
    write_private(&path, &serde_json::to_vec_pretty(&config)?)
}

fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("claudetray.tmp");
    fs::write(&temporary, data).with_context(|| format!("writing {}", temporary.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&temporary, path).with_context(|| format!("writing {}", path.display()))
}
