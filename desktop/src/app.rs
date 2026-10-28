use std::str::FromStr;
use std::thread;
use std::time::Duration;

use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::MenuEvent;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::i18n::{tr, trf};
use crate::menu::{self, Action, MenuState};
use crate::monitor::{self, Outcome, UsageMonitor};
use crate::settings::Settings;
use crate::store::Store;
use crate::{icon, platform};

const TICK: Duration = Duration::from_secs(120);

enum UserEvent {
    Menu(MenuEvent),
    TrayClicked,
    HotKey(GlobalHotKeyEvent),
    Usage(Vec<(String, Outcome)>),
    Tick,
}

struct App {
    store: Store,
    usage: UsageMonitor,
    settings: Settings,
    proxy: EventLoopProxy<UserEvent>,
    tray: Option<TrayIcon>,
    hotkeys: Option<GlobalHotKeyManager>,
    registered: Option<HotKey>,
    autostart: Option<AutoLaunch>,
}

pub fn run() -> ! {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    forward_events(event_loop.create_proxy());
    let mut app = App {
        store: Store::load(),
        usage: UsageMonitor::load(),
        settings: Settings::load(),
        proxy: event_loop.create_proxy(),
        tray: None,
        hotkeys: None,
        registered: None,
        autostart: autostart(),
    };
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => app.start(),
            Event::UserEvent(event) => {
                if app.handle(event) {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    })
}

fn forward_events(proxy: EventLoopProxy<UserEvent>) {
    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::Menu(event));
    }));
    let tray_proxy = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event| {
        if matches!(event, TrayIconEvent::Click { .. }) {
            let _ = tray_proxy.send_event(UserEvent::TrayClicked);
        }
    }));
    let hotkey_proxy = proxy.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        let _ = hotkey_proxy.send_event(UserEvent::HotKey(event));
    }));
    thread::spawn(move || loop {
        thread::sleep(TICK);
        if proxy.send_event(UserEvent::Tick).is_err() {
            break;
        }
    });
}

fn autostart() -> Option<AutoLaunch> {
    let path = std::env::current_exe().ok()?;
    AutoLaunchBuilder::new()
        .set_app_name("ClaudeTray")
        .set_app_path(path.to_str()?)
        .build()
        .ok()
}

impl App {
    fn start(&mut self) {
        self.store.sync_active();
        match TrayIconBuilder::new()
            .with_icon(self.icon())
            .with_tooltip("ClaudeTray")
            .build()
        {
            Ok(tray) => self.tray = Some(tray),
            Err(error) => platform::notify("ClaudeTray", &error.to_string()),
        }
        self.hotkeys = GlobalHotKeyManager::new().ok();
        self.apply_hotkey();
        self.refresh();
    }

    fn handle(&mut self, event: UserEvent) -> bool {
        match event {
            UserEvent::Menu(event) => match Action::parse(event.id().as_ref()) {
                Some(Action::Quit) => return true,
                Some(action) => self.perform(action),
                None => {}
            },
            UserEvent::HotKey(event) => {
                let ours = self
                    .registered
                    .is_some_and(|hotkey| hotkey.id() == event.id());
                if ours && event.state() == HotKeyState::Pressed {
                    self.switch_to_next();
                }
            }
            UserEvent::Usage(results) => {
                self.usage.finish(results);
                self.rebuild();
            }
            UserEvent::TrayClicked | UserEvent::Tick => {
                self.store.reload();
                self.refresh();
            }
        }
        false
    }

    fn perform(&mut self, action: Action) {
        match action {
            Action::Switch(id) | Action::Restore(id) => self.switch_to(&id),
            Action::Remove(id) => {
                self.store.remove(&id);
                self.usage.forget(&id);
            }
            Action::Add => {
                self.store.sync_active();
                report(platform::open_login_terminal());
            }
            Action::ToggleAutostart => {
                if let Some(autostart) = &self.autostart {
                    let enabled = autostart.is_enabled().unwrap_or(false);
                    report(if enabled {
                        autostart.disable()
                    } else {
                        autostart.enable()
                    });
                }
            }
            Action::ToggleHotKey => {
                self.settings.hotkey_enabled = !self.settings.hotkey_enabled;
                self.settings.save();
                self.apply_hotkey();
            }
            Action::Quit => {}
        }
        self.rebuild();
    }

    fn refresh(&mut self) {
        self.store.sync_active();
        self.rebuild();
        if let Some(jobs) = self
            .usage
            .begin(&self.store.accounts, self.store.active_id.as_deref())
        {
            let proxy = self.proxy.clone();
            thread::spawn(move || {
                let _ = proxy.send_event(UserEvent::Usage(monitor::fetch(jobs)));
            });
        }
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
        let Some(manager) = &self.hotkeys else {
            return;
        };
        if let Some(hotkey) = self.registered.take() {
            let _ = manager.unregister(hotkey);
        }
        if !self.settings.hotkey_enabled {
            return;
        }
        let registered = HotKey::from_str(&self.settings.hotkey)
            .ok()
            .filter(|hotkey| manager.register(*hotkey).is_ok());
        if registered.is_none() {
            platform::notify(
                "ClaudeTray",
                &tr("This shortcut is taken by another app — choose another"),
            );
        }
        self.registered = registered;
    }

    fn icon(&self) -> Icon {
        let usage = self.usage.usage(self.store.active_id.as_deref());
        let rgba = icon::render(
            usage.and_then(|usage| usage.weekly()),
            usage.and_then(|usage| usage.session()),
            platform::tray_foreground(),
        );
        Icon::from_rgba(rgba.data, rgba.size, rgba.size).expect("icon buffer matches its size")
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

    fn rebuild(&mut self) {
        let Some(tray) = &self.tray else {
            return;
        };
        let menu = menu::build(&MenuState {
            store: &self.store,
            usage: &self.usage,
            settings: &self.settings,
            autostart: self
                .autostart
                .as_ref()
                .map(|autostart| autostart.is_enabled().unwrap_or(false)),
            hotkeys_available: self.hotkeys.is_some(),
        });
        tray.set_menu(Some(Box::new(menu)));
        let _ = tray.set_icon(Some(self.icon()));
        let _ = tray.set_tooltip(Some(self.tooltip()));
    }
}

fn report<E: std::fmt::Display>(result: Result<(), E>) {
    if let Err(error) = result {
        platform::notify("ClaudeTray", &error.to_string());
    }
}
