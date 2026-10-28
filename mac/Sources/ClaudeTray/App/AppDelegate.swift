import AppKit
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private static let syncInterval: TimeInterval = 120
    private static let flashDuration: TimeInterval = 3

    private let store = AccountStore()
    private let usage = UsageMonitor()
    private var statusItem: NSStatusItem!
    private var popover: PopoverController!
    private var settingsWindow: NSWindow?
    private var syncTimer: Timer?
    private var flashing = false
    private lazy var hotKey = HotKey { [weak self] in self?.switchToNext() }

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem.button {
            button.imagePosition = .imageLeading
            button.target = self
            button.action = #selector(statusItemClicked)
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }

        popover = PopoverController(store: store, usage: usage, actions: PopoverActions(
            switchTo: { [weak self] in self?.switchTo($0) },
            restore: { [weak self] in self?.restore($0) },
            add: { [weak self] in self?.afterClosingPopover { self?.addAccount() } },
            settings: { [weak self] in self?.afterClosingPopover { self?.showSettings() } },
            quit: { NSApp.terminate(nil) }
        ))

        store.syncActive()
        refreshUsage()
        updateStatusIcon()
        updateHotKey()
        observeChanges()
        syncTimer = Timer.scheduledTimer(withTimeInterval: Self.syncInterval, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.sync() }
        }
    }

    private func observeChanges() {
        NotificationCenter.default.addObserver(forName: UserDefaults.didChangeNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.updateHotKey() }
        }
        DistributedNotificationCenter.default().addObserver(forName: .accountsChanged, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.store.reload()
                self?.store.syncActive()
                self?.updateStatusIcon()
            }
        }
    }

    private func sync() {
        store.syncActive()
        refreshUsage()
    }

    private func refreshUsage() {
        Task {
            await usage.refresh(accounts: store.accounts, activeID: store.activeID)
            updateStatusIcon()
        }
    }

    @objc private func statusItemClicked() {
        if NSApp.currentEvent?.type == .rightMouseUp {
            showContextMenu()
        } else {
            togglePopover()
        }
    }

    private func togglePopover() {
        guard let button = statusItem.button else { return }
        if popover.isShown {
            popover.close()
            return
        }
        NSApp.activate(ignoringOtherApps: true)
        popover.show(below: button)
        DispatchQueue.main.async { [weak self] in self?.sync() }
    }

    private func afterClosingPopover(_ action: @escaping @Sendable @MainActor () -> Void) {
        popover.close()
        DispatchQueue.main.async { action() }
    }

    private func showContextMenu() {
        guard let button = statusItem.button else { return }
        popover.close()
        let menu = NSMenu()
        menu.addItem(menuItem(String(localized: "Settings…"), key: ",", action: #selector(openSettings)))
        menu.addItem(.separator())
        menu.addItem(menuItem(String(localized: "Quit ClaudeTray"), key: "q", action: #selector(quit)))
        menu.popUp(positioning: nil, at: NSPoint(x: 0, y: button.bounds.height + 4), in: button)
    }

    private func menuItem(_ title: String, key: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
        item.target = self
        return item
    }

    @objc private func openSettings() {
        showSettings()
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }

    private func updateStatusIcon() {
        guard let button = statusItem.button, !flashing else { return }
        let current = usage.usage(for: store.activeID)
        let limits = [current?.weekly, current?.session].compactMap { $0 }
        button.title = ""
        button.image = StatusIcon.image(weekly: current?.weekly, session: current?.session)
        button.toolTip = limits.isEmpty
            ? String(localized: "Claude Code accounts")
            : limits.map(\.text).joined(separator: "\n")
    }

    private func flash(_ title: String, detail: String) {
        guard let button = statusItem.button else { return }
        flashing = true
        button.title = " " + title
        button.toolTip = "\(title)\n\(detail)"
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.flashDuration) { [weak self] in
            self?.flashing = false
            self?.updateStatusIcon()
        }
    }

    private func updateHotKey() {
        let wanted = Preferences.hotKeyEnabled ? Shortcut.saved : nil
        guard wanted != hotKey.shortcut else { return }
        Preferences.hotKeyFailed = !hotKey.set(wanted)
    }

    private func switchToNext() {
        store.syncActive()
        guard let next = store.nextAccount else {
            Notifier.shared.show(title: "ClaudeTray", body: String(localized: "Only one account — add another one first"))
            return
        }
        perform {
            try store.switchTo(next)
            popover.close()
            updateStatusIcon()
            let restart = String(localized: "New Claude Code sessions will use this account. Restart the ones already running.")
            Notifier.shared.show(title: String(localized: "Switched to \(next.title)"),
                                 body: (usage.lines(for: next.id).prefix(2) + [restart]).joined(separator: "\n"))
        }
    }

    private func switchTo(_ account: Account) {
        guard account.id != store.activeID else { return }
        perform {
            try store.switchTo(account)
            popover.close()
            flash(account.title, detail: String(localized: "New Claude Code sessions will use this account. Restart the ones already running."))
        }
    }

    private func restore(_ account: Account) {
        perform {
            try store.switchTo(account)
            flash(account.title, detail: String(localized: "Restart Claude Code sessions that ran under the wrong account."))
        }
    }

    private func addAccount() {
        store.syncActive()
        perform { try LoginTerminal.open() }
    }

    private func showSettings() {
        if settingsWindow == nil {
            let host = NSHostingController(rootView: SettingsView(store: store, usage: usage))
            host.sizingOptions = [.preferredContentSize]
            let window = NSWindow(contentViewController: host)
            window.title = String(localized: "ClaudeTray Settings")
            window.styleMask = [.titled, .closable]
            window.isReleasedWhenClosed = false
            window.center()
            settingsWindow = window
        }
        refreshUsage()
        NSApp.activate(ignoringOtherApps: true)
        settingsWindow?.makeKeyAndOrderFront(nil)
    }

    private func perform(_ action: () throws -> Void) {
        do {
            try action()
        } catch {
            let alert = NSAlert()
            alert.messageText = "ClaudeTray"
            alert.informativeText = error.localizedDescription
            alert.alertStyle = .warning
            NSApp.activate(ignoringOtherApps: true)
            alert.runModal()
        }
    }
}
