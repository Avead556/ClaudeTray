use chrono::{DateTime, Local, Locale, Utc};
use serde_json::Value;

use crate::i18n::{self, trf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Session,
    Weekly,
    Extra,
}

#[derive(Clone, Debug)]
pub struct Limit {
    pub kind: Kind,
    pub name: String,
    pub percent: u32,
    pub resets_at: Option<DateTime<Utc>>,
    pub weekly_scope: bool,
    pub label: String,
    pub critical: bool,
}

impl Limit {
    fn new(kind: Kind, percent: f64, resets_at: Option<DateTime<Utc>>) -> Self {
        Self {
            kind,
            name: String::new(),
            percent: percent.round().max(0.0) as u32,
            resets_at,
            weekly_scope: false,
            label: String::new(),
            critical: false,
        }
    }

    pub fn is_critical(&self) -> bool {
        self.percent >= 90 || self.critical
    }

    pub fn reset_text(&self) -> Option<String> {
        let local = self.resets_at?.with_timezone(&Local);
        let pattern = if self.kind == Kind::Session {
            "%H:%M"
        } else {
            "%a %H:%M"
        };
        Some(local.format_localized(pattern, chrono_locale()).to_string())
    }

    pub fn text(&self) -> String {
        let percent = format!("{}%", self.percent);
        match (self.kind, self.reset_text()) {
            (Kind::Session, Some(reset)) => trf("Session: %@ · resets %@", &[&percent, &reset]),
            (Kind::Session, None) => trf("Session: %@", &[&percent]),
            (Kind::Weekly, Some(reset)) => trf("Week: %@ · resets %@", &[&percent, &reset]),
            (Kind::Weekly, None) => trf("Week: %@", &[&percent]),
            (Kind::Extra, _) if self.weekly_scope => trf("%@ (week): %@", &[&self.name, &percent]),
            (Kind::Extra, _) => format!("{}: {percent}", self.name),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Usage {
    pub limits: Vec<Limit>,
}

impl Usage {
    pub fn from_json(json: &Value) -> Self {
        let mut usage = Self::default();
        for item in json["limits"].as_array().into_iter().flatten() {
            let Some(percent) = item["percent"].as_f64() else {
                continue;
            };
            let mut limit = Limit::new(Kind::Extra, percent, parse_date(&item["resets_at"]));
            limit.critical = item["severity"] == "critical";
            let kind = item["kind"].as_str().unwrap_or_default();
            let weekly = item["group"] == "weekly";
            if let Some(model) = item["scope"]["model"]["display_name"].as_str() {
                limit.name = model.to_owned();
                limit.weekly_scope = weekly;
            } else if kind == "session" && usage.session().is_none() {
                limit.kind = Kind::Session;
            } else if weekly && usage.weekly().is_none() {
                limit.kind = Kind::Weekly;
            } else {
                limit.name = capitalized(kind.split('_').next().unwrap_or(kind));
            }
            usage.limits.push(limit);
        }
        if usage.limits.is_empty() {
            for (kind, key) in [(Kind::Session, "five_hour"), (Kind::Weekly, "seven_day")] {
                if let Some(percent) = json[key]["utilization"].as_f64() {
                    usage.limits.push(Limit::new(
                        kind,
                        percent,
                        parse_date(&json[key]["resets_at"]),
                    ));
                }
            }
        }
        usage.assign_labels();
        usage
    }

    pub fn session(&self) -> Option<&Limit> {
        self.limits.iter().find(|limit| limit.kind == Kind::Session)
    }

    pub fn weekly(&self) -> Option<&Limit> {
        self.limits.iter().find(|limit| limit.kind == Kind::Weekly)
    }

    pub fn lines(&self) -> Vec<String> {
        self.limits.iter().map(Limit::text).collect()
    }

    fn assign_labels(&mut self) {
        let names: Vec<Vec<char>> = self
            .limits
            .iter()
            .filter(|limit| limit.kind == Kind::Extra)
            .map(|limit| limit.name.chars().collect())
            .collect();
        for limit in self
            .limits
            .iter_mut()
            .filter(|limit| limit.kind == Kind::Extra)
        {
            limit.label = short_label(&limit.name.chars().collect::<Vec<_>>(), &names);
        }
    }
}

fn short_label(name: &[char], all: &[Vec<char>]) -> String {
    let Some(first) = name.first() else {
        return String::new();
    };
    let same_first = |other: &Vec<char>| {
        other.as_slice() != name
            && other
                .first()
                .map(|c| c.to_lowercase().eq(first.to_lowercase()))
                == Some(true)
    };
    let rivals: Vec<&Vec<char>> = all.iter().filter(|other| same_first(other)).collect();
    let mut label: String = first.to_uppercase().collect();
    if !rivals.is_empty() && name.len() > 1 {
        let differs = (1..name.len()).find(|&i| {
            rivals.iter().all(|rival| {
                rival
                    .get(i)
                    .is_none_or(|c| !c.to_lowercase().eq(name[i].to_lowercase()))
            })
        });
        label.extend(name[differs.unwrap_or(name.len() - 1)].to_lowercase());
    }
    label
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn parse_date(value: &Value) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

fn chrono_locale() -> Locale {
    let name = match i18n::language() {
        "ru" => "ru_RU",
        "uk" => "uk_UA",
        "de" => "de_DE",
        "fr" => "fr_FR",
        "es" => "es_ES",
        "it" => "it_IT",
        "pt-BR" => "pt_BR",
        "pl" => "pl_PL",
        "tr" => "tr_TR",
        "ja" => "ja_JP",
        "ko" => "ko_KR",
        "zh-Hans" => "zh_CN",
        _ => "en_US",
    };
    Locale::try_from(name).unwrap_or(Locale::en_US)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn models(names: &[&str]) -> Usage {
        let limits: Vec<Value> = names
            .iter()
            .map(|name| json!({"kind": "weekly_scoped", "group": "weekly", "percent": 1.0, "scope": {"model": {"display_name": name}}}))
            .collect();
        Usage::from_json(&json!({ "limits": limits }))
    }

    #[test]
    fn reads_limits_list() {
        let usage = Usage::from_json(&json!({"limits": [
            {"kind": "session", "group": "session", "percent": 14.4, "resets_at": "2026-10-28T01:00:00Z"},
            {"kind": "weekly_all", "group": "weekly", "percent": 91.0, "severity": "critical"},
            {"kind": "daily_all", "group": "daily", "percent": 3.0},
        ]}));
        assert_eq!(usage.session().map(|limit| limit.percent), Some(14));
        assert!(usage.weekly().is_some_and(Limit::is_critical));
        assert_eq!(usage.limits[2].name, "Daily");
    }

    #[test]
    fn falls_back_to_legacy_fields() {
        let usage = Usage::from_json(
            &json!({"five_hour": {"utilization": 3.0}, "seven_day": {"utilization": 90.0}}),
        );
        assert_eq!(usage.limits.len(), 2);
        assert!(usage.weekly().is_some_and(Limit::is_critical));
    }

    #[test]
    fn disambiguates_labels() {
        let labels: Vec<String> = models(&["Fable", "Falcon", "Opus"])
            .limits
            .into_iter()
            .map(|l| l.label)
            .collect();
        assert_eq!(labels, ["Fb", "Fl", "O"]);
    }
}
