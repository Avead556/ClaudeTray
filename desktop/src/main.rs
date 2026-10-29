#![cfg_attr(windows, windows_subsystem = "windows")]

mod account;
mod api;
mod app;
mod claude_code;
mod cli;
mod i18n;
mod icon;
mod monitor;
mod paths;
mod platform;
mod settings;
mod store;
mod usage;
mod vault;
mod view;

mod ui {
    slint::include_modules!();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if cli::requested(&args) {
        platform::attach_console();
        std::process::exit(cli::run(&args));
    }
    app::run()
}
