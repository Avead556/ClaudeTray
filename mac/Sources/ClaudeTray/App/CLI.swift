import Foundation

enum CLI {
    static let commands: Set = ["list", "status", "switch", "next", "help", "--help", "-h", "--usage"]

    static var requested: Bool {
        CommandLine.arguments.dropFirst().first.map(commands.contains) ?? false
    }

    @MainActor
    static func run() async -> Int32 {
        let args = Array(CommandLine.arguments.dropFirst())
        let store = AccountStore()
        let usage = UsageMonitor()
        store.syncActive()

        switch args.first {
        case "switch":
            let query = args.dropFirst().prefix { !$0.hasPrefix("-") }.joined(separator: " ")
            guard !query.isEmpty else { return printUsage() }
            switch store.account(matching: query) {
            case .found(let account):
                return switchTo(account, store: store)
            case .none:
                printError(String(localized: "No account matches “\(query)”"))
                list(store, usage: usage)
                return 1
            case .several(let accounts):
                let names = accounts.map(\.title).joined(separator: ", ")
                printError(String(localized: "Several accounts match “\(query)”: \(names)"))
                return 1
            }
        case "next":
            guard let next = store.nextAccount else {
                printError(String(localized: "Only one account — add another one first"))
                return 1
            }
            return switchTo(next, store: store)
        case "help", "--help", "-h":
            _ = printUsage()
            return 0
        default:
            await usage.refresh(accounts: store.accounts, activeID: store.activeID)
            list(store, usage: usage)
            return 0
        }
    }

    @MainActor
    private static func switchTo(_ account: Account, store: AccountStore) -> Int32 {
        guard account.id != store.activeID else {
            print(String(localized: "Already on \(account.title)"))
            return 0
        }
        do {
            try store.switchTo(account)
        } catch {
            printError(error.localizedDescription)
            return 1
        }
        DistributedNotificationCenter.default().postNotificationName(.accountsChanged, object: nil,
                                                                     userInfo: nil, deliverImmediately: true)
        print("✓ " + String(localized: "Switched to \(account.title)"))
        print("  " + String(localized: "New Claude Code sessions will use this account. Restart the ones already running."))
        return 0
    }

    @MainActor
    private static func list(_ store: AccountStore, usage: UsageMonitor) {
        for (index, account) in store.accounts.enumerated() {
            let mark = account.id == store.activeID ? "✓" : " "
            let email = account.nickname != nil ? "  \(account.email)" : ""
            print("\(mark) \(index + 1). \(account.title)\(email)  \(account.plan)")
            for line in usage.lines(for: account.id) { print("       " + line) }
        }
    }

    private static func printUsage() -> Int32 {
        print(String(localized: "Usage: claudetray [list | switch <number, name or email> | next]"))
        return 2
    }

    private static func printError(_ message: String) {
        FileHandle.standardError.write(Data((message + "\n").utf8))
    }
}
