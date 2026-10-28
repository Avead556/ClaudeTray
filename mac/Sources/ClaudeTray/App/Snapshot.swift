import AppKit
import SwiftUI

enum Snapshot {
    static var requested: Bool { path != nil }

    private static var path: String? {
        let arguments = CommandLine.arguments
        guard let flag = arguments.firstIndex(of: "--snapshot"), flag + 1 < arguments.count else { return nil }
        return arguments[flag + 1]
    }

    @MainActor
    static func run() async -> Int32 {
        guard let path else { return 2 }
        let arguments = CommandLine.arguments
        let demo = arguments.contains("--demo")
        let usage = UsageMonitor()
        let store: AccountStore
        if demo {
            let accounts = demoAccounts
            store = AccountStore(preview: accounts, activeID: accounts.first?.id)
            for (account, limits) in zip(accounts, demoLimits) { usage.preview(limits, for: account.id) }
        } else {
            store = AccountStore()
            store.syncActive()
            await usage.refresh(accounts: store.accounts, activeID: store.activeID)
        }
        let actions = PopoverActions(switchTo: { _ in }, restore: { _ in }, add: {}, settings: {}, quit: {})
        let content = arguments.contains("--settings")
            ? AnyView(SettingsView(store: store, usage: usage))
            : AnyView(PopoverView(store: store, usage: usage, actions: actions, expandAll: demo))
        let host = NSHostingView(rootView: content.background(Color(white: 0.17)).environment(\.colorScheme, .dark))
        host.appearance = NSAppearance(named: .darkAqua)
        if !arguments.contains("--settings") { host.frame.size.width = 300 }
        host.frame.size = host.fittingSize
        let window = NSWindow(contentRect: host.frame, styleMask: .borderless, backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        host.display()
        guard let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds) else { return 1 }
        host.cacheDisplay(in: host.bounds, to: bitmap)
        guard let png = bitmap.representation(using: .png, properties: [:]),
              (try? png.write(to: URL(fileURLWithPath: path))) != nil else { return 1 }
        return 0
    }

    private static var demoAccounts: [Account] {
        let people: [(String, String, String, String, String?, Int?)] = [
            ("alex@example.com", "Alex Morgan", "", "claude_max", "default_claude_max_5x", nil),
            ("alex@acme.example", "Alex Morgan", "Acme Labs", "claude_team", nil, 2),
            ("side.project@example.org", "", "", "claude_pro", nil, 4),
        ]
        return people.enumerated().compactMap { index, person in
            let (email, name, organization, type, tier, color) = person
            let info: [String: Any] = [
                "accountUuid": "demo-\(index)", "organizationUuid": "demo-org-\(index)", "emailAddress": email,
                "displayName": name, "organizationName": organization, "organizationType": type,
                "organizationRateLimitTier": tier ?? "",
            ]
            guard let json = try? JSONSerialization.data(withJSONObject: info),
                  var account = Account(oauthAccount: json) else { return nil }
            account.nickname = index == 1 ? "Work" : nil
            account.color = color
            return account
        }
    }

    private static var demoLimits: [[String: Any]] {
        let date: (TimeInterval) -> String = { ISO8601DateFormatter().string(from: Date().addingTimeInterval($0)) }
        let models = [("Fable", 64), ("Model X", 30), ("Model Y", 5)]
        let scoped: [[String: Any]] = models.map { name, percent in
            ["kind": "weekly_scoped", "group": "weekly", "percent": Double(percent),
             "resets_at": date(200_000), "scope": ["model": ["display_name": name]]]
        }
        let account: (Double, Double, Bool) -> [String: Any] = { session, weekly, withModels in
            let base: [[String: Any]] = [
                ["kind": "session", "group": "session", "percent": session, "resets_at": date(8_400)],
                ["kind": "weekly_all", "group": "weekly", "percent": weekly, "resets_at": date(200_000)],
            ]
            return ["limits": base + (withModels ? scoped : [])]
        }
        return [account(37, 91, true), account(12, 48, false), account(0, 7, false)]
    }
}
