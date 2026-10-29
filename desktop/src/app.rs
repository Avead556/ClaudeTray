use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use slint::platform::Key;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{
    CloseRequestResponse, ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer, SharedString,
    Timer, TimerMode, VecModel,
};

use crate::i18n::{tr, trf};
use crate::monitor::{self, Outcome, UsageMonitor};
use crate::settings::Settings;
use crate::store::{Issue, Store};
use crate::ui::{AccountItem, Tr, Tray};
use crate::{icon, platform, view};

#[cfg(not(windows))]
use crate::ui::{LinuxPopover as Popover, LinuxSettings as SettingsWindow};
#[cfg(windows)]
use crate::ui::{WinPopover as Popover, WinSettings as SettingsWindow};

const TICK: Duration = Duration::from_secs(120);
const REOPEN_GUARD: Duration = Duration::from_millis(350);

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app(action: impl FnOnce(&mut App) + 'static) {
    let pending = APP.with(|cell| match cell.try_borrow_mut() {
        Ok(mut app) => {
            if let Some(app) = app.as_mut() {
                action(app);
            }
            None
        }
        Err(_) => Some(action),
    });
    if let Some(action) = pending {
        Timer::single_shot(Duration::ZERO, move || with_app(action));
    }
}

fn from_any_thread(action: impl FnOnce(&mut App) + Send + 'static) {
    let _ = slint::invoke_from_event_loop(move || with_app(action));
}

struct App {
    store: Store,
    usage: UsageMonitor,
    settings: Settings,
    tray: Tray,
    popover: Popover,
    settings_window: SettingsWindow,
    accounts: Rc<VecModel<AccountItem>>,
    hotkeys: Option<GlobalHotKeyManager>,
    registered: Option<HotKey>,
    hotkey_failed: bool,
    autostart: Option<AutoLaunch>,
    popover_shown: bool,
    hidden_at: Option<Instant>,
    anchor: Option<(i32, i32)>,
    _tick: Timer,
}

pub fn run() {
    if let Err(error) = platform::select_backend() {
        platform::notify("ClaudeTray", &error.to_string());
    }
    let app = match App::new() {
        Ok(app) => app,
        Err(error) => {
            platform::notify("ClaudeTray", &error.to_string());
            return;
        }
    };
    APP.with(|cell| *cell.borrow_mut() = Some(app));
    with_app(App::start);
    let _ = slint::run_event_loop_until_quit();
}

impl App {
    fn new() -> Result<Self, slint::PlatformError> {
        let tray = Tray::new()?;
        let popover = Popover::new()?;
        let settings_window = SettingsWindow::new()?;
        view::strings(&tray.global::<Tr>());
        view::strings(&popover.global::<Tr>());
        view::strings(&settings_window.global::<Tr>());

        let accounts = Rc::new(VecModel::default());
        popover.set_accounts(accounts.clone().into());
        settings_window.set_accounts(accounts.clone().into());
        settings_window.set_palette(view::palette());
        settings_window.set_linux_x11_note(platform::hotkey_needs_x11());

        let tick = Timer::default();
        tick.start(TimerMode::Repeated, TICK, || {
            with_app(|app| {
                app.store.reload();
                app.refresh();
            })
        });

        Ok(Self {
            store: Store::load(),
            usage: UsageMonitor::load(),
            settings: Settings::load(),
            tray,
            popover,
            settings_window,
            accounts,
            hotkeys: None,
            registered: None,
            hotkey_failed: false,
            autostart: autostart(),
            popover_shown: false,
            hidden_at: None,
            anchor: None,
            _tick: tick,
        })
    }

    fn start(&mut self) {
        self.connect_tray();
        self.connect_popover();
        self.connect_settings();
        GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
            from_any_thread(move |app| app.hotkey_pressed(event));
        }));
        self.hotkeys = GlobalHotKeyManager::new().ok();
        self.apply_hotkey();
        self.refresh();
    }

    fn connect_tray(&self) {
        self.tray.on_activated(|| with_app(App::toggle_popover));
        self.tray.on_show_accounts(|| {
            with_app(|app| {
                app.anchor = platform::cursor();
                app.show_popover();
            })
        });
        self.tray.on_open_settings(|| with_app(App::show_settings));
        self.tray.on_quit(quit);
    }

    fn connect_popover(&self) {
        let popover = &self.popover;
        popover.on_switch(|id| with_app(move |app| app.switch_to(&id)));
        popover.on_restore(|| with_app(App::restore));
        popover.on_add(|| with_app(App::add_account));
        popover.on_settings(|| with_app(App::show_settings));
        popover.on_quit(quit);
        popover.on_key(|text| {
            let digit = text
                .parse::<usize>()
                .ok()
                .filter(|digit| (1..=9).contains(digit));
            if text == SharedString::from(Key::Escape) {
                with_app(App::hide_popover);
                true
            } else if let Some(digit) = digit {
                with_app(move |app| {
                    if let Some(id) = app.store.accounts.get(digit - 1).map(|a| a.id.clone()) {
                        app.switch_to(&id);
                    }
                });
                true
            } else {
                false
            }
        });
        popover.on_resized(|| with_app(App::place_popover));
        popover.window().on_close_requested(|| {
            with_app(App::hide_popover);
            CloseRequestResponse::HideWindow
        });
        popover.window().on_winit_window_event(|_, event| {
            if platform::is_focus_lost(event) {
                with_app(App::hide_popover);
            }
            EventResult::Propagate
        });
    }

    fn connect_settings(&self) {
        let window = &self.settings_window;
        window.on_add(|| with_app(App::add_account));
        window.on_remove(|id| {
            with_app(move |app| {
                app.store.remove(&id);
                app.usage.forget(&id);
                app.rebuild();
            })
        });
        window.on_rename(|id, name| {
            with_app(move |app| {
                app.store.rename(&id, &name);
                app.rebuild();
            })
        });
        window.on_set_color(|id, index| {
            with_app(move |app| {
                app.store.set_color(
                    &id,
                    usize::try_from(index)
                        .ok()
                        .filter(|&i| i < view::PALETTE.len()),
                );
                app.rebuild();
            })
        });
        window.on_toggle_autostart(|| with_app(App::toggle_autostart));
        window.on_toggle_hotkey(|| {
            with_app(|app| {
                app.settings.hotkey_enabled = !app.settings.hotkey_enabled;
                app.settings.save();
                app.apply_hotkey();
                app.rebuild();
            })
        });
        window.on_record_hotkey(|text, control, alt, shift, meta| {
            let Some(hotkey) = hotkey_code(&text, control, alt, shift, meta) else {
                return false;
            };
            with_app(move |app| {
                app.settings.hotkey = hotkey;
                app.settings.hotkey_enabled = true;
                app.settings.save();
                app.apply_hotkey();
                app.rebuild();
            });
            true
        });
        window.on_quit(quit);
    }

    fn toggle_popover(&mut self) {
        if self.popover_shown {
            self.hide_popover();
        } else if self
            .hidden_at
            .is_none_or(|hidden| hidden.elapsed() > REOPEN_GUARD)
        {
            self.anchor = platform::cursor();
            self.show_popover();
        }
    }

    fn show_popover(&mut self) {
        self.store.reload();
        self.refresh();
        self.place_popover();
        if self.popover.show().is_ok() {
            self.popover_shown = true;
            self.place_popover();
            platform::focus_popover(self.popover.window());
        }
    }

    fn place_popover(&mut self) {
        platform::place_popover(
            self.popover.window(),
            self.anchor,
            self.popover
                .window()
                .size()
                .to_logical(self.popover.window().scale_factor())
                .width,
            self.popover.get_content_height(),
        );
    }

    fn hide_popover(&mut self) {
        if self.popover_shown {
            self.popover_shown = false;
            self.hidden_at = Some(Instant::now());
            let _ = self.popover.hide();
        }
    }

    fn show_settings(&mut self) {
        self.hide_popover();
        self.refresh();
        let _ = self.settings_window.show();
        self.settings_window
            .window()
            .with_winit_window(|window| window.focus_window());
    }

    fn add_account(&mut self) {
        self.hide_popover();
        self.store.sync_active();
        report(platform::open_login_terminal());
    }

    fn restore(&mut self) {
        let configured = match &self.store.issue {
            Some(Issue::TokenMismatch { configured, .. }) => configured.id.clone(),
            _ => return,
        };
        self.switch_to(&configured);
    }

    fn toggle_autostart(&mut self) {
        if let Some(autostart) = &self.autostart {
            let enabled = autostart.is_enabled().unwrap_or(false);
            report(if enabled {
                autostart.disable()
            } else {
                autostart.enable()
            });
        }
        self.rebuild();
    }

    fn hotkey_pressed(&mut self, event: GlobalHotKeyEvent) {
        let ours = self
            .registered
            .is_some_and(|hotkey| hotkey.id() == event.id());
        if ours && event.state() == HotKeyState::Pressed {
            self.switch_to_next();
        }
    }

    fn refresh(&mut self) {
        self.store.sync_active();
        self.rebuild();
        if let Some(jobs) = self
            .usage
            .begin(&self.store.accounts, self.store.active_id.as_deref())
        {
            thread::spawn(move || {
                let results = monitor::fetch(jobs);
                from_any_thread(move |app| app.finish_usage(results));
            });
        }
    }

    fn finish_usage(&mut self, results: Vec<(String, Outcome)>) {
        self.usage.finish(results);
        self.rebuild();
    }

    fn switch_to(&mut self, id: &str) {
        let Some(title) = self
            .store
            .account(id)
            .map(|account| account.title().to_owned())
        else {
            return;
        };
        match self.store.switch_to(id) {
            Ok(()) => platform::notify(
                &trf("Switched to %@", &[&title]),
                &tr("New Claude Code sessions will use this account. Restart the ones already running."),
            ),
            Err(error) => platform::notify("ClaudeTray", &error.to_string()),
        }
        self.rebuild();
    }

    fn switch_to_next(&mut self) {
        self.store.sync_active();
        match self.store.next_account().map(|next| next.id.clone()) {
            Some(id) => self.switch_to(&id),
            None => platform::notify(
                "ClaudeTray",
                &tr("Only one account — add another one first"),
            ),
        }
        self.rebuild();
    }

    fn apply_hotkey(&mut self) {
        self.hotkey_failed = false;
        let Some(manager) = &self.hotkeys else {
            return;
        };
        if let Some(hotkey) = self.registered.take() {
            let _ = manager.unregister(hotkey);
        }
        if !self.settings.hotkey_enabled {
            return;
        }
        self.registered = HotKey::from_str(&self.settings.hotkey)
            .ok()
            .filter(|hotkey| manager.register(*hotkey).is_ok());
        self.hotkey_failed = self.registered.is_none();
    }

    fn rebuild(&mut self) {
        let active = self.store.active_id.as_deref();
        let items = self
            .store
            .accounts
            .iter()
            .map(|account| {
                view::account(
                    account,
                    active == Some(account.id.as_str()),
                    self.usage.state(&account.id),
                )
            })
            .collect();
        view::sync(&self.accounts, items);

        let (issue, restore) = match &self.store.issue {
            Some(issue) => {
                let restore = match issue {
                    Issue::TokenMismatch { configured, .. }
                        if self.store.account(&configured.id).is_some() =>
                    {
                        trf("Restore %@", &[&configured.email])
                    }
                    _ => String::new(),
                };
                (issue.message(), restore)
            }
            None => (String::new(), String::new()),
        };
        self.popover.set_issue(issue.into());
        self.popover.set_restore_label(restore.into());
        let hotkey_title = self.settings.hotkey_title();
        self.popover.set_hotkey(
            if self.settings.hotkey_enabled && self.registered.is_some() {
                hotkey_title.as_str()
            } else {
                ""
            }
            .into(),
        );

        let window = &self.settings_window;
        window.set_autostart_available(self.autostart.is_some());
        window.set_autostart(
            self.autostart
                .as_ref()
                .is_some_and(|autostart| autostart.is_enabled().unwrap_or(false)),
        );
        window.set_hotkeys_available(self.hotkeys.is_some());
        window.set_hotkey_enabled(self.settings.hotkey_enabled);
        window.set_hotkey_keys(view::hotkey_keys(&hotkey_title));
        window.set_hotkey_failed(self.hotkey_failed);

        self.tray.set_image(self.icon());
        self.tray.set_tip(self.tooltip().into());
    }

    fn icon(&self) -> Image {
        let usage = self.usage.usage(self.store.active_id.as_deref());
        let rgba = icon::render(
            usage.and_then(|usage| usage.weekly()),
            usage.and_then(|usage| usage.session()),
            platform::tray_foreground(),
        );
        Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            &rgba.data, rgba.size, rgba.size,
        ))
    }

    fn tooltip(&self) -> String {
        let lines = self
            .store
            .active_id
            .as_deref()
            .map(|id| self.usage.lines(id))
            .unwrap_or_default();
        if lines.is_empty() {
            tr("Claude Code accounts")
        } else {
            lines.into_iter().take(2).collect::<Vec<_>>().join("\n")
        }
    }
}

fn quit() {
    let _ = slint::quit_event_loop();
}

fn autostart() -> Option<AutoLaunch> {
    let path = std::env::current_exe().ok()?;
    AutoLaunchBuilder::new()
        .set_app_name("ClaudeTray")
        .set_app_path(path.to_str()?)
        .build()
        .ok()
}

fn report<E: std::fmt::Display>(result: Result<(), E>) {
    if let Err(error) = result {
        platform::notify("ClaudeTray", &error.to_string());
    }
}

fn hotkey_code(text: &str, control: bool, alt: bool, shift: bool, meta: bool) -> Option<String> {
    if !(control || alt || meta) {
        return None;
    }
    let mut chars = text.chars();
    let typed = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let function = [
        Key::F1,
        Key::F2,
        Key::F3,
        Key::F4,
        Key::F5,
        Key::F6,
        Key::F7,
        Key::F8,
        Key::F9,
        Key::F10,
        Key::F11,
        Key::F12,
    ]
    .into_iter()
    .position(|key| char::from(key) == typed);
    let key = if let Some(index) = function {
        format!("F{}", index + 1)
    } else if let Some(digit) = digit(typed) {
        format!("Digit{digit}")
    } else {
        format!("Key{}", latin_letter(typed)?)
    };
    let modifiers = [
        (control, "ctrl"),
        (alt, "alt"),
        (shift, "shift"),
        (meta, "super"),
    ];
    let mut parts: Vec<String> = modifiers
        .into_iter()
        .filter(|(pressed, _)| *pressed)
        .map(|(_, name)| name.to_owned())
        .collect();
    parts.push(key);
    Some(parts.join("+"))
}

fn digit(typed: char) -> Option<char> {
    if typed.is_ascii_digit() {
        return Some(typed);
    }
    let shifted = ")!@#$%^&*(";
    shifted
        .chars()
        .position(|c| c == typed)
        .and_then(|index| char::from_digit(index as u32, 10))
}

fn latin_letter(typed: char) -> Option<char> {
    const RUSSIAN: &str = "йцукенгшщзфывапролдячсмить";
    const LATIN: &str = "QWERTYUIOPASDFGHJKLZXCVBNM";
    let lower = typed.to_lowercase().next()?;
    if lower.is_ascii_lowercase() {
        return Some(lower.to_ascii_uppercase());
    }
    let index = RUSSIAN.chars().position(|c| c == lower)?;
    LATIN.chars().nth(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_hotkeys() {
        assert_eq!(
            hotkey_code("A", true, true, true, false).as_deref(),
            Some("ctrl+alt+shift+KeyA")
        );
        assert_eq!(
            hotkey_code("ф", true, false, false, false).as_deref(),
            Some("ctrl+KeyA")
        );
        assert_eq!(
            hotkey_code("!", true, false, true, false).as_deref(),
            Some("ctrl+shift+Digit1")
        );
        assert_eq!(hotkey_code("a", false, false, true, false), None);
        let title = Settings {
            hotkey: hotkey_code("ф", true, true, false, false).unwrap(),
            ..Settings::default()
        }
        .hotkey_title();
        assert_eq!(title, "Ctrl+Alt+A");
    }
}
