import Foundation

@MainActor
enum AnthropicAPI {
    enum Failure: Error {
        case unauthorized
        case rateLimited
        case other
    }

    private static let usageURL = URL(string: "https://api.anthropic.com/api/oauth/usage")!
    private static let tokenURL = URL(string: "https://platform.claude.com/v1/oauth/token")!
    private static let clientID = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"

    static func usage(accessToken: String) async throws -> [String: Any] {
        var request = URLRequest(url: usageURL, timeoutInterval: 10)
        request.setValue("Bearer \(accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("oauth-2025-04-20", forHTTPHeaderField: "anthropic-beta")
        return try await send(request)
    }

    static func refresh(oauth: [String: Any]) async throws -> [String: Any] {
        guard let refreshToken = oauth["refreshToken"] as? String else { throw Failure.unauthorized }
        var request = URLRequest(url: tokenURL, timeoutInterval: 10)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: [
            "grant_type": "refresh_token", "refresh_token": refreshToken, "client_id": clientID,
        ])
        let json = try await send(request)
        guard let access = json["access_token"] as? String else { throw Failure.other }
        var updated = oauth
        updated["accessToken"] = access
        if let refresh = json["refresh_token"] as? String { updated["refreshToken"] = refresh }
        if let expiresIn = json["expires_in"] as? Double {
            updated["expiresAt"] = Int((Date().timeIntervalSince1970 + expiresIn) * 1000)
        }
        return updated
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
