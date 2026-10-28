import Foundation

@MainActor
enum AnthropicAPI {
    enum Failure: Error {
        case unauthorized
        case rateLimited
        case other
    }

    private static let usageURL = URL(string: "https://api.anthropic.com/api/oauth/usage")!

    static func usage(accessToken: String) async throws -> [String: Any] {
        var request = URLRequest(url: usageURL, timeoutInterval: 10)
        request.setValue("Bearer \(accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("oauth-2025-04-20", forHTTPHeaderField: "anthropic-beta")
        return try await send(request)
    }

    static func isExpired(_ oauth: [String: Any]) -> Bool {
        guard let expiresAt = oauth["expiresAt"] as? Double else { return true }
        return expiresAt / 1000 < Date().timeIntervalSince1970 + 60
    }

    private static func send(_ request: URLRequest) async throws -> [String: Any] {
        let (body, response) = try await URLSession.shared.data(for: request)
        switch (response as? HTTPURLResponse)?.statusCode {
        case 200: break
        case 400, 401, 403: throw Failure.unauthorized
        case 429: throw Failure.rateLimited
        default: throw Failure.other
        }
        guard let json = try JSONSerialization.jsonObject(with: body) as? [String: Any] else { throw Failure.other }
        return json
    }
}
