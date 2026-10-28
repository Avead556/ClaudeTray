use std::io::Write;

use crate::account::Account;
use crate::i18n::{tr, trf};
use crate::monitor::{self, UsageMonitor};
use crate::store::{Match, Store};

const COMMANDS: [&str; 7] = [
    "list", "status", "switch", "next", "rename", "help", "--help",
];

pub fn requested(args: &[String]) -> bool {
    args.first()
        .is_some_and(|command| COMMANDS.contains(&command.as_str()))
}

pub fn run(args: &[String]) -> i32 {
    let mut store = Store::load();
    let mut usage = UsageMonitor::load();
    store.sync_active();
    let words: Vec<&str> = args
        .iter()
        .skip(1)
        .map(String::as_str)
        .take_while(|arg| !arg.starts_with('-'))
        .collect();

    match args.first().map(String::as_str) {
        Some("switch") => match resolve(&store, &usage, &words.join(" ")) {
            Some(id) => switch(&mut store, &id),
            None => 1,
        },
        Some("next") => match store.next_account().map(|next| next.id.clone()) {
            Some(id) => switch(&mut store, &id),
            None => fail(&tr("Only one account — add another one first")),
        },
        Some("rename") if words.len() >= 2 => match resolve(&store, &usage, words[0]) {
            Some(id) => {
                store.rename(&id, &words[1..].join(" "));
                list(&store, &usage);
                0
            }
            None => 1,
        },
        Some("help" | "--help" | "rename") => {
            println!("{}", usage_text());
            0
        }
        _ => {
            if let Some(jobs) = usage.begin(&store.accounts, store.active_id.as_deref()) {
                usage.finish(monitor::fetch(jobs));
            }
            list(&store, &usage);
            0
        }
    }
}

fn resolve(store: &Store, usage: &UsageMonitor, query: &str) -> Option<String> {
    if query.is_empty() {
        println!("{}", usage_text());
        return None;
    }
    match store.find(query) {
        Match::Found(account) => Some(account.id.clone()),
        Match::None => {
            fail(&trf("No account matches “%@”", &[query]));
            list(store, usage);
            None
        }
        Match::Several(accounts) => {
            let names: Vec<&str> = accounts.iter().map(|account| account.title()).collect();
            fail(&trf(
                "Several accounts match “%@”: %@",
                &[query, &names.join(", ")],
            ));
            None
        }
    }
}

fn switch(store: &mut Store, id: &str) -> i32 {
    let Some(account) = store.account(id).cloned() else {
        return 1;
    };
    if store.is_active(&account) {
        println!("{}", trf("Already on %@", &[account.title()]));
        return 0;
    }
    if let Err(error) = store.switch_to(id) {
        return fail(&error.to_string());
    }
    println!("✓ {}", trf("Switched to %@", &[account.title()]));
    println!(
        "  {}",
        tr("New Claude Code sessions will use this account. Restart the ones already running.")
    );
    0
}

fn list(store: &Store, usage: &UsageMonitor) {
    for (index, account) in store.accounts.iter().enumerate() {
        let mark = if store.is_active(account) { "✓" } else { " " };
        println!(
            "{mark} {}. {}  {}",
            index + 1,
            describe(account),
            account.plan
        );
        for line in usage.lines(&account.id) {
            println!("       {line}");
        }
    }
}

fn describe(account: &Account) -> String {
    match &account.nickname {
        Some(nickname) => format!("{nickname}  {}", account.email),
        None => account.email.clone(),
    }
}

fn usage_text() -> String {
    [
        "Usage: claudetray [list | switch <number, name or email> | next]",
        "Rename: claudetray rename <number, name or email> <new name>",
    ]
    .map(tr)
    .join("\n")
}

fn fail(message: &str) -> i32 {
    let _ = writeln!(std::io::stderr(), "{message}");
    1
}
