use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime};

use serde_json::{Map, Value};

use crate::account::Account;
use crate::api::{self, ApiError};
use crate::i18n::tr;
use crate::usage::Usage;
use crate::{claude_code, paths, vault};

const INTERVAL: Duration = Duration::from_secs(180);
const RATE_LIMIT_PAUSE: Duration = Duration::from_secs(600);

pub enum UsageState {
    Loaded(Usage),
    SignedOut,
    Unavailable,
}

pub struct Job {
    id: String,
    active: bool,
    known: bool,
}

pub enum Outcome {
    Loaded(Value),
    SignedOut,
    Unavailable,
    Keep,
    RateLimited,
}

#[derive(Default)]
pub struct UsageMonitor {
    states: HashMap<String, UsageState>,
    cache: Map<String, Value>,
    fetched_at: Option<SystemTime>,
    paused_until: Option<SystemTime>,
    fetching: bool,
}

impl UsageMonitor {
    pub fn load() -> Self {
        let mut monitor = Self::default();
        let cache = fs::read(paths::usage())
            .ok()
            .and_then(|data| serde_json::from_slice::<Map<String, Value>>(&data).ok());
        if let Some(cache) = cache {
            monitor.states = cache
                .iter()
                .map(|(id, json)| (id.clone(), UsageState::Loaded(Usage::from_json(json))))
                .collect();
            monitor.cache = cache;
            monitor.fetched_at = fs::metadata(paths::usage())
                .and_then(|meta| meta.modified())
                .ok();
        }
        monitor
    }

    pub fn usage(&self, id: Option<&str>) -> Option<&Usage> {
        match self.states.get(id?) {
            Some(UsageState::Loaded(usage)) => Some(usage),
            _ => None,
        }
    }

    pub fn state(&self, id: &str) -> Option<&UsageState> {
        self.states.get(id)
    }

    pub fn lines(&self, id: &str) -> Vec<String> {
        match self.states.get(id) {
            Some(UsageState::Loaded(usage)) => usage.lines(),
            Some(UsageState::SignedOut) => vec![tr("Login expired — add the account again")],
            Some(UsageState::Unavailable) => vec![tr("Limits unavailable")],
            None => Vec::new(),
        }
    }

    pub fn forget(&mut self, id: &str) {
        self.states.remove(id);
        self.cache.remove(id);
    }

    pub fn begin(&mut self, accounts: &[Account], active_id: Option<&str>) -> Option<Vec<Job>> {
        let now = SystemTime::now();
        let due = self
            .fetched_at
            .is_none_or(|at| now.duration_since(at).unwrap_or_default() > INTERVAL);
        let paused = self.paused_until.is_some_and(|until| until > now);
        if self.fetching || !due || paused {
            return None;
        }
        self.fetching = true;
        let jobs = accounts
            .iter()
            .map(|account| Job {
                id: account.id.clone(),
                active: active_id == Some(account.id.as_str()),
                known: self.states.contains_key(&account.id),
            })
            .collect();
        Some(jobs)
    }

    pub fn finish(&mut self, results: Vec<(String, Outcome)>) {
        self.fetching = false;
        self.fetched_at = Some(SystemTime::now());
        for (id, outcome) in results {
            let state = match outcome {
                Outcome::Loaded(json) => {
                    let usage = Usage::from_json(&json);
                    self.cache.insert(id.clone(), json);
                    UsageState::Loaded(usage)
                }
                Outcome::SignedOut => UsageState::SignedOut,
                Outcome::Unavailable => UsageState::Unavailable,
                Outcome::Keep => continue,
                Outcome::RateLimited => {
                    self.paused_until = Some(SystemTime::now() + RATE_LIMIT_PAUSE);
                    continue;
                }
            };
            self.states.insert(id, state);
        }
        paths::ensure_directory();
        if let Ok(data) = serde_json::to_vec(&self.cache) {
            let _ = fs::write(paths::usage(), data);
        }
    }
}

pub fn fetch(jobs: Vec<Job>) -> Vec<(String, Outcome)> {
    let mut results = Vec::with_capacity(jobs.len());
    for job in jobs {
        let outcome = fetch_one(&job);
        let rate_limited = matches!(outcome, Outcome::RateLimited);
        results.push((job.id, outcome));
        if rate_limited {
            break;
        }
    }
    results
}

fn fetch_one(job: &Job) -> Outcome {
    let saved = if job.active {
        claude_code::read_oauth()
    } else {
        vault::token(&job.id)
    };
    let Some(saved) = saved else {
        return Outcome::Unavailable;
    };
    if api::is_expired(&saved) {
        return if job.known {
            Outcome::Keep
        } else {
            Outcome::Unavailable
        };
    }

    let Some(token) = api::access_token(&saved) else {
        return Outcome::Unavailable;
    };
    match api::usage(token) {
        Ok(json) => Outcome::Loaded(json),
        Err(ApiError::Unauthorized) if !job.active => Outcome::SignedOut,
        Err(ApiError::RateLimited) => Outcome::RateLimited,
        Err(_) => Outcome::Keep,
    }
}
