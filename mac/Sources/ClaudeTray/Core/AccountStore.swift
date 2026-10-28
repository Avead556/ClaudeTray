import Foundation
import Observation

@MainActor
@Observable
final class AccountStore {
    enum Issue {
        case tokenMismatch(configured: Account, owner: Account?)
        case unverified(Account)
    }

    enum Match {
        case found(Account)
        case none
        case several([Account])
    }

    private(set) var accounts: [Account] = []
    private(set) var activeID: String?
    private(set) var issue: Issue?

    init() {
        reload()
    }

    init(preview accounts: [Account], activeID: String?) {
        self.accounts = accounts
        self.activeID = activeID
    }

    var active: Account? { accounts.first { $0.id == activeID } }

    var nextAccount: Account? {
        guard accounts.count > 1 else { return nil }
        let index = accounts.firstIndex { $0.id == activeID } ?? -1
        return accounts[(index + 1) % accounts.count]
    }

    func reload() {
        guard let data = try? Data(contentsOf: AppPaths.accounts),
              let saved = try? JSONDecoder().decode([Account].self, from: data) else { return }
        accounts = saved
    }

    func syncActive() {
        issue = nil
        guard let json = ClaudeCode.readAccount(), let current = Account(oauthAccount: json),
              let oauth = ClaudeCode.readOAuth() else {
            activeID = nil
            return
        }
        activeID = current.id

        if TokenVault.token(for: current.id) == oauth {
            upsert(current)
            return
        }

        switch TokenOwner.check(oauth: oauth) {
        case .account(let owner) where owner == current.id:
            if save(oauth, for: current.id) { upsert(current) }
        case .account(let ownerID):
            let owner = accounts.first { $0.id == ownerID }
            if owner != nil { save(oauth, for: ownerID) }
            issue = .tokenMismatch(configured: current, owner: owner)
        case .unknown:
            issue = .unverified(current)
        }
    }

    func switchTo(_ account: Account) throws {
        syncActive()
        guard let oauth = TokenVault.token(for: account.id) else {
            throw AppError(String(localized: "No saved login for \(account.email). Add the account again."))
        }
        try ClaudeCode.writeOAuth(oauth)
        try ClaudeCode.writeAccount(account.oauthAccount)
        activeID = account.id
        issue = nil
    }

    func remove(_ account: Account) {
        TokenVault.delete(account.id)
        accounts.removeAll { $0.id == account.id }
        persist()
    }

    func setNickname(_ nickname: String, for id: String) {
        let trimmed = nickname.trimmingCharacters(in: .whitespacesAndNewlines)
        update(id) { $0.nickname = trimmed.isEmpty ? nil : trimmed }
    }

    func setColor(_ color: Int?, for id: String) {
        update(id) { $0.color = color }
    }

    func account(matching query: String) -> Match {
        if let number = Int(query), accounts.indices.contains(number - 1) { return .found(accounts[number - 1]) }
        let query = query.lowercased()
        let names: (Account) -> [String] = { [$0.nickname, $0.email].compactMap { $0?.lowercased() } }
        let tests: [(String) -> Bool] = [{ $0 == query }, { $0.hasPrefix(query) }, { $0.contains(query) }]
        for test in tests {
            let found = accounts.filter { names($0).contains(where: test) }
            if found.count == 1 { return .found(found[0]) }
            if found.count > 1 { return .several(found) }
        }
        return .none
    }

    private func update(_ id: String, _ change: (inout Account) -> Void) {
        guard let index = accounts.firstIndex(where: { $0.id == id }) else { return }
        change(&accounts[index])
        persist()
    }

    @discardableResult
    private func save(_ oauth: String, for id: String) -> Bool {
        (try? TokenVault.save(oauth, for: id)) != nil
    }

    private func upsert(_ account: Account) {
        guard let index = accounts.firstIndex(where: { $0.id == account.id }) else {
            accounts.append(account)
            persist()
            return
        }
        var account = account
        account.nickname = accounts[index].nickname
        account.color = accounts[index].color
        guard accounts[index] != account else { return }
        accounts[index] = account
        persist()
    }

    private func persist() {
        guard let data = try? JSONEncoder().encode(accounts) else { return }
        try? data.write(to: AppPaths.accounts, options: .atomic)
    }
}

extension AccountStore.Issue {
    var message: String {
        switch self {
        case .tokenMismatch(let configured, let owner):
            if let owner {
                String(localized: "Claude Code is set to \(configured.email), but its token belongs to \(owner.email). An old session probably refreshed it after the switch. Close old sessions and restore the login.")
            } else {
                String(localized: "Claude Code is set to \(configured.email), but its token belongs to another account. An old session probably refreshed it after the switch. Close old sessions and restore the login.")
            }
        case .unverified(let account):
            String(localized: "Couldn’t verify the current login for \(account.email) (offline or expired token). It isn’t saved yet — run Claude Code and open the menu again.")
        }
    }
}
