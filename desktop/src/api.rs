use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};

const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const BETA: &str = "oauth-2025-04-20";

#[derive(Debug, PartialEq, Eq)]
pub enum ApiError {
    Unauthorized,
    RateLimited,
    Other,
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(timeout).build()
}

fn send(request: ureq::Request) -> Result<Value, ApiError> {
    match request.call() {
        Ok(response) => response.into_json().map_err(|_| ApiError::Other),
        Err(ureq::Error::Status(400 | 401 | 403, _)) => Err(ApiError::Unauthorized),
        Err(ureq::Error::Status(429, _)) => Err(ApiError::RateLimited),
        Err(_) => Err(ApiError::Other),
    }
}

fn authorized_get(url: &str, token: &str, timeout: Duration) -> Result<Value, ApiError> {
    let request = agent(timeout)
        .get(url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("anthropic-beta", BETA);
    send(request)
}

pub fn access_token(oauth: &Value) -> Option<&str> {
    oauth["accessToken"].as_str()
}

pub fn owner(oauth: &Value) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = format!("{:x}", Sha256::digest(oauth.to_string()));
    if let Some(id) = cache.lock().ok()?.get(&key) {
        return Some(id.clone());
    }
    let profile = authorized_get(PROFILE_URL, access_token(oauth)?, Duration::from_secs(3)).ok()?;
    let id = format!(
        "{}:{}",
        profile["account"]["uuid"].as_str()?,
        profile["organization"]["uuid"].as_str()?
    );
    cache.lock().ok()?.insert(key, id.clone());
    Some(id)
}

pub fn usage(token: &str) -> Result<Value, ApiError> {
    authorized_get(USAGE_URL, token, Duration::from_secs(10))
}

pub fn is_expired(oauth: &Value) -> bool {
    oauth["expiresAt"]
        .as_f64()
        .is_none_or(|expires| expires / 1000.0 < now_seconds() + 60.0)
}

fn now_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
