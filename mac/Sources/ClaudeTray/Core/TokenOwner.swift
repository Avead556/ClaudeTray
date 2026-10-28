import CryptoKit
import Foundation

@MainActor
enum TokenOwner {
    enum Result: Equatable {
        case account(String)
        case unknown
    }

    private static let endpoint = URL(string: "https://api.anthropic.com/api/oauth/profile")!
    private static var cache: [String: String] = [:]

    static func check(oauth: String) -> Result {
        let key = SHA256.hash(data: Data(oauth.utf8)).map { String(format: "%02x", $0) }.joined()
        if let id = cache[key] { return .account(id) }
        guard let token = ClaudeCode.accessToken(inOAuth: oauth) else { return .unknown }

        var request = URLRequest(url: endpoint, timeoutInterval: 3)
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("oauth-2025-04-20", forHTTPHeaderField: "anthropic-beta")

        nonisolated(unsafe) var result = Result.unknown
        let done = DispatchSemaphore(value: 0)
        URLSession.shared.dataTask(with: request) { body, response, _ in
            defer { done.signal() }
            guard (response as? HTTPURLResponse)?.statusCode == 200, let body,
                  let profile = try? JSONSerialization.jsonObject(with: body) as? [String: Any],
                  let account = (profile["account"] as? [String: Any])?["uuid"] as? String,
                  let org = (profile["organization"] as? [String: Any])?["uuid"] as? String else { return }
            result = .account("\(account):\(org)")
        }.resume()
        done.wait()

        if case .account(let id) = result { cache[key] = id }
        return result
    }
}
