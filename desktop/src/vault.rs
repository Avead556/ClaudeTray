use anyhow::Result;
use serde_json::Value;

const SERVICE: &str = "ClaudeTray";

fn entry(id: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(SERVICE, id)?)
}

pub fn token(id: &str) -> Option<Value> {
    serde_json::from_str(&entry(id).ok()?.get_password().ok()?).ok()
}

pub fn save(id: &str, oauth: &Value) -> Result<()> {
    entry(id)?.set_password(&oauth.to_string())?;
    Ok(())
}

pub fn delete(id: &str) {
    if let Ok(entry) = entry(id) {
        let _ = entry.delete_credential();
    }
}
