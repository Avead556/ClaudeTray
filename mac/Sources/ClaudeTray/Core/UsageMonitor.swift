import Foundation
import Observation

@MainActor
@Observable
final class UsageMonitor {
    enum State {
        case loaded(Usage)
        case signedOut
        case unavailable
    }

    private static let interval: TimeInterval = 180
    private static let rateLimitPause: TimeInterval = 600

    private(set) var states: [String: State] = [:]
    @ObservationIgnored private var cache: [String: [String: Any]] = [:]
    @ObservationIgnored private var fetchedAt: Date?
    @ObservationIgnored private var pausedUntil: Date?
    @ObservationIgnored private var fetching = false

    init() {
        guard let data = try? Data(contentsOf: AppPaths.usage),
              let cache = try? JSONSerialization.jsonObject(with: data) as? [String: [String: Any]] else { return }
        self.cache = cache
        states = cache.mapValues { .loaded(Usage(json: $0)) }
        fetchedAt = (try? FileManager.default.attributesOfItem(atPath: AppPaths.usage.path))?[.modificationDate] as? Date
    }

    func usage(for id: String?) -> Usage? {
        guard let id, case .loaded(let usage)? = states[id] else { return nil }
        return usage
    }

    func lines(for id: String) -> [String] {
        switch states[id] {
        case .loaded(let usage): usage.lines
        case .signedOut: [String(localized: "Login expired — add the account again")]
        case .unavailable: [String(localized: "Limits unavailable")]
        case nil: []
        }
    }

    func refresh(accounts: [Account], activeID: String?) async {
        let now = Date()
        guard !fetching, now.timeIntervalSince(fetchedAt ?? .distantPast) > Self.interval,
              now > pausedUntil ?? .distantPast else { return }
        fetching = true
        defer {
            fetching = false
            fetchedAt = Date()
        }
        for account in accounts {
            if let state = await fetch(account, active: account.id == activeID) { states[account.id] = state }
            if let pausedUntil, pausedUntil > Date() { break }
        }
        if let data = try? JSONSerialization.data(withJSONObject: cache) {
            try? data.write(to: AppPaths.usage, options: .atomic)
        }
    }

    func forget(_ id: String) {
        states[id] = nil
        cache[id] = nil
    }

    func preview(_ json: [String: Any], for id: String) {
        states[id] = .loaded(Usage(json: json))
    }

    private func fetch(_ account: Account, active: Bool) async -> State? {
        guard let saved = active ? ClaudeCode.readOAuth() : TokenVault.token(for: account.id),
              let oauth = ClaudeCode.object(saved) else { return .unavailable }

        if AnthropicAPI.isExpired(oauth) { return states[account.id] == nil ? .unavailable : nil }

        guard let token = oauth["accessToken"] as? String else { return .unavailable }
        do {
            let json = try await AnthropicAPI.usage(accessToken: token)
            cache[account.id] = json
            return .loaded(Usage(json: json))
        } catch AnthropicAPI.Failure.unauthorized {
            return active ? nil : .signedOut
        } catch AnthropicAPI.Failure.rateLimited {
            pausedUntil = Date().addingTimeInterval(Self.rateLimitPause)
            return nil
        } catch {
            return nil
        }
    }
}
