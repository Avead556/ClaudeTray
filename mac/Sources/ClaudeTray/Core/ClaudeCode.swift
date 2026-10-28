import Foundation

enum ClaudeCode {
    static let credentialsService = "Claude Code-credentials"
    private static let oauthKey = "claudeAiOauth"

    static var configURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".claude.json")
            .resolvingSymlinksInPath()
    }


    static func readOAuth() -> String? {
        guard let blob = readCredentials(), let oauth = blob[oauthKey] else { return nil }
        return json(oauth)
    }

    static func writeOAuth(_ oauth: String) throws {
        guard let value = parse(oauth) else { throw AppError(String(localized: "Saved tokens are corrupted")) }
        var blob = readCredentials() ?? [:]
        blob[oauthKey] = value
        guard let secret = json(blob) else { throw AppError(String(localized: "Couldn’t build the token JSON")) }
        let account = Keychain.account(service: credentialsService) ?? NSUserName()
        try Keychain.write(service: credentialsService, account: account, secret: secret)
    }

    static func accessToken(inOAuth oauth: String) -> String? {
        (parse(oauth) as? [String: Any])?["accessToken"] as? String
    }

    static func object(_ oauth: String) -> [String: Any]? { parse(oauth) as? [String: Any] }

    static func string(_ oauth: [String: Any]) -> String? { json(oauth) }

    private static func readCredentials() -> [String: Any]? {
        Keychain.read(service: credentialsService).flatMap(parse) as? [String: Any]
    }


    private static func loadConfig() throws -> [String: Any] {
        guard let config = parse(try Data(contentsOf: configURL)) as? [String: Any] else {
            throw AppError(String(localized: "~/.claude.json is not a JSON object"))
        }
        return config
    }

    static func readAccount() -> Data? {
        guard let config = try? loadConfig(), let account = config["oauthAccount"] as? [String: Any] else { return nil }
        return try? JSONSerialization.data(withJSONObject: account)
    }

    static func writeAccount(_ account: Data) throws {
        var config = try loadConfig()
        config["oauthAccount"] = parse(account)
        let data = try JSONSerialization.data(withJSONObject: config, options: [.prettyPrinted, .withoutEscapingSlashes])
        try data.write(to: configURL, options: .atomic)
    }


    private static func parse(_ string: String) -> Any? { parse(Data(string.utf8)) }

    private static func parse(_ data: Data) -> Any? { try? JSONSerialization.jsonObject(with: data) }

    private static func json(_ object: Any) -> String? {
        guard let data = try? JSONSerialization.data(withJSONObject: object, options: [.withoutEscapingSlashes]) else { return nil }
        return String(decoding: data, as: UTF8.self)
    }
}
