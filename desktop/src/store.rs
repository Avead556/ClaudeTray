use std::fs;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::account::Account;
use crate::i18n::trf;
use crate::{api, claude_code, paths, vault};

pub enum Issue {
    TokenMismatch {
        configured: Box<Account>,
        owner: Option<Box<Account>>,
    },
    Unverified(Box<Account>),
}

impl Issue {
    pub fn message(&self) -> String {
        match self {
            Issue::TokenMismatch { configured, owner: Some(owner) } => trf(
                "Claude Code is set to %@, but its token belongs to %@. An old session probably refreshed it after the switch. Close old sessions and restore the login.",
                &[&configured.email, &owner.email],
            ),
            Issue::TokenMismatch { configured, owner: None } => trf(
                "Claude Code is set to %@, but its token belongs to another account. An old session probably refreshed it after the switch. Close old sessions and restore the login.",
                &[&configured.email],
            ),
            Issue::Unverified(account) => trf(
                "Couldn’t verify the current login for %@ (offline or expired token). It isn’t saved yet — run Claude Code and open the menu again.",
                &[&account.email],
            ),
        }
    }
}

pub enum Match<'a> {
    Found(&'a Account),
    None,
    Several(Vec<&'a Account>),
}

#[derive(Default)]
pub struct Store {
    pub accounts: Vec<Account>,
    pub active_id: Option<String>,
    pub issue: Option<Issue>,
}

impl Store {
    pub fn load() -> Self {
        let mut store = Self::default();
        store.reload();
        store
    }

    pub fn reload(&mut self) {
        if let Some(accounts) = fs::read(paths::accounts())
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
        {
            self.accounts = accounts;
        }
    }

    pub fn account(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|account| account.id == id)
    }

    pub fn is_active(&self, account: &Account) -> bool {
        self.active_id.as_deref() == Some(account.id.as_str())
    }

    pub fn sync_active(&mut self) {
        self.issue = None;
        let current = claude_code::read_account()
            .as_ref()
            .and_then(Account::from_oauth_account);
        let (Some(current), Some(oauth)) = (current, claude_code::read_oauth()) else {
            self.active_id = None;
            return;
        };
        self.active_id = Some(current.id.clone());

        if vault::token(&current.id).as_ref() == Some(&oauth) {
            self.upsert(current);
            return;
        }
        match api::owner(&oauth) {
            Some(owner) if owner == current.id => {
                if vault::save(&current.id, &oauth).is_ok() {
                    self.upsert(current);
                }
            }
            Some(owner_id) => self.record_mismatch(current, &owner_id, &oauth),
            None => self.issue = Some(Issue::Unverified(Box::new(current))),
        }
    }

    fn record_mismatch(&mut self, configured: Account, owner_id: &str, oauth: &Value) {
        let owner = self.account(owner_id).cloned();
        if owner.is_some() {
            let _ = vault::save(owner_id, oauth);
        }
        self.issue = Some(Issue::TokenMismatch {
            configured: Box::new(configured),
            owner: owner.map(Box::new),
        });
    }

    pub fn switch_to(&mut self, id: &str) -> Result<()> {
        self.sync_active();
        let account = self
            .account(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown account"))?;
        let oauth = vault::token(id).ok_or_else(|| {
            anyhow!(trf(
                "No saved login for %@. Add the account again.",
                &[&account.email]
            ))
        })?;
        claude_code::write_oauth(&oauth)?;
        claude_code::write_account(&account.oauth_account)?;
        self.active_id = Some(account.id);
        self.issue = None;
        Ok(())
    }

    pub fn remove(&mut self, id: &str) {
        vault::delete(id);
        self.accounts.retain(|account| account.id != id);
        self.persist();
    }

    pub fn rename(&mut self, id: &str, nickname: &str) {
        let nickname = nickname.trim();
        if let Some(account) = self.accounts.iter_mut().find(|account| account.id == id) {
            account.nickname = (!nickname.is_empty()).then(|| nickname.to_owned());
            self.persist();
        }
    }

    pub fn next_account(&self) -> Option<&Account> {
        if self.accounts.len() < 2 {
            return None;
        }
        let next = self
            .accounts
            .iter()
            .position(|account| self.is_active(account))
            .map_or(0, |index| index + 1);
        self.accounts.get(next % self.accounts.len())
    }

    pub fn find(&self, query: &str) -> Match<'_> {
        let by_number = query
            .parse::<usize>()
            .ok()
            .and_then(|number| self.accounts.get(number.checked_sub(1)?));
        if let Some(account) = by_number {
            return Match::Found(account);
        }
        let query = query.to_lowercase();
        let tests: [&dyn Fn(&str) -> bool; 3] = [
            &|name| name == query,
            &|name| name.starts_with(&query),
            &|name| name.contains(&query),
        ];
        for test in tests {
            let found: Vec<&Account> = self
                .accounts
                .iter()
                .filter(|account| {
                    [account.nickname.as_deref(), Some(account.email.as_str())]
                        .into_iter()
                        .flatten()
                        .any(|name| test(&name.to_lowercase()))
                })
                .collect();
            match found.as_slice() {
                [] => continue,
                [single] => return Match::Found(single),
                _ => return Match::Several(found),
            }
        }
        Match::None
    }

    fn upsert(&mut self, mut account: Account) {
        match self
            .accounts
            .iter_mut()
            .find(|existing| existing.id == account.id)
        {
            Some(existing) => {
                account.nickname = existing.nickname.clone();
                if *existing == account {
                    return;
                }
                *existing = account;
            }
            None => self.accounts.push(account),
        }
        self.persist();
    }

    fn persist(&self) {
        paths::ensure_directory();
        if let Ok(data) = serde_json::to_vec_pretty(&self.accounts) {
            let _ = fs::write(paths::accounts(), data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn account(email: &str, nickname: Option<&str>) -> Account {
        let mut account =
            Account::from_oauth_account(&json!({"accountUuid": email, "emailAddress": email}))
                .unwrap();
        account.nickname = nickname.map(str::to_owned);
        account
    }

    #[test]
    fn finds_accounts() {
        let store = Store {
            accounts: vec![
                account("a@gmail.com", Some("Work")),
                account("a@proton.me", None),
            ],
            ..Store::default()
        };
        assert!(matches!(store.find("2"), Match::Found(found) if found.email == "a@proton.me"));
        assert!(matches!(store.find("work"), Match::Found(found) if found.email == "a@gmail.com"));
        assert!(matches!(store.find("a@"), Match::Several(found) if found.len() == 2));
        assert!(matches!(store.find("yahoo"), Match::None));
    }

    #[test]
    fn cycles_to_next_account() {
        let mut store = Store {
            accounts: vec![account("a", None), account("b", None)],
            ..Store::default()
        };
        store.active_id = Some(store.accounts[1].id.clone());
        assert_eq!(
            store.next_account().map(|next| next.email.as_str()),
            Some("a")
        );
    }
}
