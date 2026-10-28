use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

use crate::i18n::{tr, trf};
use crate::monitor::UsageMonitor;
use crate::settings::Settings;
use crate::store::{Issue, Store};

pub enum Action {
    Switch(String),
    Restore(String),
    Remove(String),
    Add,
    ToggleAutostart,
    ToggleHotKey,
    Quit,
}

impl Action {
    fn id(&self) -> String {
        match self {
            Action::Switch(id) => format!("switch:{id}"),
            Action::Restore(id) => format!("restore:{id}"),
            Action::Remove(id) => format!("remove:{id}"),
            Action::Add => "add".into(),
            Action::ToggleAutostart => "autostart".into(),
            Action::ToggleHotKey => "hotkey".into(),
            Action::Quit => "quit".into(),
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        let (kind, account) = id.split_once(':').unwrap_or((id, ""));
        Some(match kind {
            "switch" => Action::Switch(account.into()),
            "restore" => Action::Restore(account.into()),
            "remove" => Action::Remove(account.into()),
            "add" => Action::Add,
            "autostart" => Action::ToggleAutostart,
            "hotkey" => Action::ToggleHotKey,
            "quit" => Action::Quit,
            _ => return None,
        })
    }
}

pub struct MenuState<'a> {
    pub store: &'a Store,
    pub usage: &'a UsageMonitor,
    pub settings: &'a Settings,
    pub autostart: Option<bool>,
    pub hotkeys_available: bool,
}

pub fn build(state: &MenuState) -> Menu {
    let menu = Menu::new();
    let add = |item: &dyn IsMenuItem| {
        let _ = menu.append(item);
    };
    let store = state.store;

    add(&label(&tr("Claude Code accounts")));
    if let Some(issue) = &store.issue {
        add(&label(&format!("⚠ {}", issue.message())));
        if let Issue::TokenMismatch { configured, .. } = issue {
            if store.account(&configured.id).is_some() {
                add(&action(
                    Action::Restore(configured.id.clone()),
                    &trf("Restore %@", &[&configured.email]),
                ));
            }
        }
        add(&PredefinedMenuItem::separator());
    }

    if store.accounts.is_empty() {
        add(&label(&tr("No accounts yet — sign in to Claude Code")));
    }
    for account in &store.accounts {
        let active = store.is_active(account);
        let title = match account.plan.as_str() {
            "" => account.title().to_owned(),
            plan => format!("{}  ·  {plan}", account.title()),
        };
        add(&CheckMenuItem::with_id(
            Action::Switch(account.id.clone()).id(),
            title,
            !active,
            active,
            None,
        ));
        for line in state.usage.lines(&account.id) {
            add(&label(&format!("      {line}")));
        }
    }

    add(&PredefinedMenuItem::separator());
    add(&action(Action::Add, &format!("{}…", tr("Add account"))));
    let remove = Submenu::new(tr("Remove account"), !store.accounts.is_empty());
    for account in &store.accounts {
        let id = Action::Remove(account.id.clone()).id();
        let _ = remove.append(&MenuItem::with_id(
            id,
            account.title(),
            !store.is_active(account),
            None,
        ));
    }
    add(&remove);

    add(&PredefinedMenuItem::separator());
    add(&CheckMenuItem::with_id(
        Action::ToggleAutostart.id(),
        tr("Open at login"),
        state.autostart.is_some(),
        state.autostart.unwrap_or(false),
        None,
    ));
    add(&CheckMenuItem::with_id(
        Action::ToggleHotKey.id(),
        format!(
            "{} ({})",
            tr("Switch to the next account"),
            state.settings.hotkey_title()
        ),
        state.hotkeys_available,
        state.settings.hotkey_enabled,
        None,
    ));
    add(&PredefinedMenuItem::separator());
    add(&action(Action::Quit, &tr("Quit ClaudeTray")));
    menu
}

fn label(text: &str) -> MenuItem {
    MenuItem::new(text, false, None)
}

fn action(action: Action, text: &str) -> MenuItem {
    MenuItem::with_id(action.id(), text, true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_action_ids() {
        let id = Action::Switch("account:org".into()).id();
        assert!(
            matches!(Action::parse(&id), Some(Action::Switch(account)) if account == "account:org")
        );
        assert!(matches!(Action::parse("quit"), Some(Action::Quit)));
        assert!(Action::parse("unknown").is_none());
    }
}
