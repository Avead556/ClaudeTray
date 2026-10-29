use slint::{Color, Model, ModelRc, SharedString, VecModel};

use crate::account::Account;
use crate::i18n::{tr, trf};
use crate::monitor::UsageState;
use crate::ui::{AccountItem, LimitRow, Ring, Tr};
use crate::usage::{Kind, Limit, Usage};

pub const PALETTE: [[u8; 3]; 9] = [
    [94, 92, 230],
    [10, 132, 255],
    [255, 159, 10],
    [64, 200, 224],
    [255, 55, 95],
    [100, 210, 255],
    [255, 69, 58],
    [255, 214, 10],
    [142, 142, 147],
];

pub fn palette() -> ModelRc<Color> {
    ModelRc::new(VecModel::from(
        PALETTE.iter().map(|&rgb| color(rgb)).collect::<Vec<_>>(),
    ))
}

pub fn strings(strings: &Tr) {
    strings.set_title(tr("Claude Code accounts").into());
    strings.set_add_account(tr("Add account").into());
    strings.set_add(tr("Add").into());
    strings.set_settings(tr("Settings").into());
    strings.set_settings_menu(tr("Settings…").into());
    strings.set_settings_title(tr("ClaudeTray Settings").into());
    strings.set_quit(tr("Quit ClaudeTray").into());
    strings.set_accounts(tr("Accounts").into());
    strings.set_general(tr("General").into());
    strings.set_open_at_login(tr("Open at login").into());
    strings.set_switch_next(tr("Switch to the next account").into());
    strings.set_press_shortcut(tr("Press a shortcut…").into());
    strings.set_shortcut_taken(tr("This shortcut is taken by another app — choose another").into());
    strings.set_on(tr("On").into());
    strings.set_off(tr("Off").into());
    strings.set_empty(tr("No accounts yet — sign in to Claude Code").into());
    strings.set_active(tr("Active").into());
    strings.set_active_account(tr("Active account").into());
    strings.set_remove_account(tr("Remove account").into());
    strings.set_remove(tr("Remove").into());
    strings.set_cancel(tr("Cancel").into());
    strings.set_remove_message(
        tr("The saved login will be removed from ClaudeTray. To use the account again, sign in to it again.").into(),
    );
    strings.set_show_limits(tr("Show limits").into());
    strings.set_hide_limits(tr("Hide limits").into());
    strings.set_session(tr("Session").into());
    strings.set_week(tr("Week").into());
    strings.set_default_color(tr("Default color").into());
    strings.set_login_expired(tr("Login expired — add the account again").into());
    strings.set_limits_unavailable(tr("Limits unavailable").into());
    strings.set_show_accounts(tr("Show accounts").into());
    strings.set_x11_only(tr("X11 only").into());
}

pub fn account(account: &Account, active: bool, state: Option<&UsageState>) -> AccountItem {
    let usage = match state {
        Some(UsageState::Loaded(usage)) => Some(usage),
        _ => None,
    };
    let rgb = avatar_color(account);
    let label = if account.nickname.is_some() {
        account.email.clone()
    } else if account.organization.is_empty() {
        account.name.clone()
    } else {
        account.organization.clone()
    };
    let details = [
        if account.nickname.is_some() {
            account.email.as_str()
        } else {
            ""
        },
        &details(account),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" · ");
    let session = usage.and_then(Usage::session);
    let weekly = usage.and_then(Usage::weekly);

    AccountItem {
        id: account.id.as_str().into(),
        title: account.title().into(),
        label: label.into(),
        email: account.email.as_str().into(),
        nickname: account.nickname.clone().unwrap_or_default().into(),
        summary: summary(&details, session, weekly).into(),
        details: details.into(),
        initials: initials(account).into(),
        color: color(rgb),
        initials_color: if luminance(rgb) > 0.6 {
            Color::from_rgb_u8(0, 0, 0)
        } else {
            Color::from_rgb_u8(255, 255, 255)
        },
        active,
        plan: plan_title(&account.plan).into(),
        plan_kind: plan_kind(&account.plan),
        state: match state {
            None => 0,
            Some(UsageState::Loaded(_)) => 1,
            Some(UsageState::SignedOut) => 2,
            Some(UsageState::Unavailable) => 3,
        },
        rings: model(rings(usage)),
        limits: model(
            usage
                .map(|usage| usage.limits.iter().map(limit_row).collect())
                .unwrap_or_default(),
        ),
        session_percent: session.map_or(0, |limit| limit.percent as i32),
        session_critical: session.is_some_and(Limit::is_critical),
        week_percent: weekly.map_or(0, |limit| limit.percent as i32),
        week_critical: weekly.is_some_and(Limit::is_critical),
        remove_title: trf("Remove %@?", &[&account.email]).into(),
    }
}

pub fn sync<T: Clone + 'static>(model: &VecModel<T>, items: Vec<T>) {
    let count = items.len();
    for (index, item) in items.into_iter().enumerate() {
        if index < model.row_count() {
            model.set_row_data(index, item);
        } else {
            model.push(item);
        }
    }
    while model.row_count() > count {
        model.remove(model.row_count() - 1);
    }
}

pub fn hotkey_keys(title: &str) -> ModelRc<SharedString> {
    model(title.split('+').map(SharedString::from).collect())
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

fn rings(usage: Option<&Usage>) -> Vec<Ring> {
    let Some(usage) = usage else {
        return vec![empty_ring(0), empty_ring(1)];
    };
    let mut rings: Vec<Ring> = [usage.session(), usage.weekly()]
        .into_iter()
        .flatten()
        .map(|limit| ring(limit, limit.percent.to_string(), 0))
        .collect();
    let extras: Vec<&Limit> = usage
        .limits
        .iter()
        .filter(|limit| limit.kind == Kind::Extra)
        .collect();
    if let Some(top) = extras.iter().max_by_key(|limit| limit.percent) {
        rings.push(ring(top, top.label.clone(), extras.len() as i32 - 1));
    }
    rings
}

fn ring(limit: &Limit, label: String, more: i32) -> Ring {
    Ring {
        percent: limit.percent as i32,
        label: label.into(),
        kind: kind(limit.kind),
        critical: limit.is_critical(),
        more,
    }
}

fn empty_ring(kind: i32) -> Ring {
    Ring {
        kind,
        ..Ring::default()
    }
}

fn limit_row(limit: &Limit) -> LimitRow {
    let title = match limit.kind {
        Kind::Session => tr("Session"),
        Kind::Weekly => tr("Week"),
        Kind::Extra if limit.weekly_scope => trf("%@ (week)", &[&limit.name]),
        Kind::Extra => limit.name.clone(),
    };
    LimitRow {
        title: title.into(),
        reset: limit
            .reset_text()
            .map(|reset| trf("Resets %@", &[&reset]))
            .unwrap_or_default()
            .into(),
        percent: limit.percent as i32,
        kind: kind(limit.kind),
        critical: limit.is_critical(),
    }
}

fn kind(kind: Kind) -> i32 {
    match kind {
        Kind::Session => 0,
        Kind::Weekly => 1,
        Kind::Extra => 2,
    }
}

fn details(account: &Account) -> String {
    [account.name.as_str(), account.organization.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn summary(details: &str, session: Option<&Limit>, weekly: Option<&Limit>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !details.is_empty() {
        parts.push(details.to_owned());
    }
    for (name, limit) in [(tr("Session"), session), (tr("Week"), weekly)] {
        if let Some(limit) = limit {
            parts.push(format!("{} {}%", name.to_lowercase(), limit.percent));
        }
    }
    parts.join(" · ")
}

fn initials(account: &Account) -> String {
    let source = account.nickname.as_deref().unwrap_or(&account.name);
    let words: Vec<&str> = source.split_whitespace().collect();
    let letters: String = if words.len() >= 2 {
        words
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect()
    } else if account.nickname.is_some() {
        source.chars().take(2).collect()
    } else {
        account.email.chars().take(2).collect()
    };
    letters.to_uppercase()
}

fn plan_title(plan: &str) -> String {
    match plan.strip_prefix("Max") {
        Some(rest) => format!("MAX{rest}"),
        None => plan.to_owned(),
    }
}

fn plan_kind(plan: &str) -> i32 {
    if plan.starts_with("Max") {
        1
    } else if plan.starts_with("Team") || plan.starts_with("Enterprise") {
        2
    } else {
        0
    }
}

fn avatar_color(account: &Account) -> [u8; 3] {
    if let Some(&rgb) = account.color.and_then(|index| PALETTE.get(index)) {
        return rgb;
    }
    let sum: usize = account.id.chars().map(|c| c as usize).sum();
    PALETTE[sum % 6]
}

fn color([r, g, b]: [u8; 3]) -> Color {
    Color::from_rgb_u8(r, g, b)
}

fn luminance([r, g, b]: [u8; 3]) -> f32 {
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn account(name: &str, nickname: Option<&str>) -> Account {
        let mut account = Account::from_oauth_account(&json!({
            "accountUuid": "a", "emailAddress": "dalk@example.com", "displayName": name
        }))
        .unwrap();
        account.nickname = nickname.map(str::to_owned);
        account
    }

    #[test]
    fn builds_initials() {
        assert_eq!(initials(&account("Daniil Pitertsev", None)), "DP");
        assert_eq!(initials(&account("Avead", None)), "DA");
        assert_eq!(initials(&account("Avead", Some("work"))), "WO");
    }

    #[test]
    fn styles_plans() {
        assert_eq!(plan_title("Max 5x"), "MAX 5x");
        assert_eq!(plan_kind("Max 5x"), 1);
        assert_eq!(plan_kind("Team"), 2);
        assert_eq!(plan_kind("Pro"), 0);
    }

    #[test]
    fn shows_empty_rings_without_usage() {
        let rings = rings(None);
        assert_eq!(rings.len(), 2);
        assert!(rings.iter().all(|ring| ring.percent == 0));
    }
}
