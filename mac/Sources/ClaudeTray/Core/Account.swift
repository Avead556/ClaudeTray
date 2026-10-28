import Foundation

struct Account: Codable, Equatable {
    var id: String
    var email: String
    var name: String
    var organization: String
    var plan: String
    var oauthAccount: Data
    var nickname: String?
    var color: Int?

    var title: String { nickname ?? email }

    init?(oauthAccount json: Data) {
        guard let info = try? JSONSerialization.jsonObject(with: json) as? [String: Any],
              let accountUuid = info["accountUuid"] as? String else { return nil }
        id = "\(accountUuid):\(info["organizationUuid"] as? String ?? "")"
        email = info["emailAddress"] as? String ?? "—"
        name = info["displayName"] as? String ?? info["fullName"] as? String ?? ""
        organization = info["organizationName"] as? String ?? ""
        if organization == "\(email)'s Organization" { organization = "" }
        plan = Self.plan(type: info["organizationType"] as? String, tier: info["organizationRateLimitTier"] as? String)
        oauthAccount = json
    }

    var details: String {
        [name, organization].filter { !$0.isEmpty && $0 != email }.joined(separator: " · ")
    }

    private static func plan(type: String?, tier: String?) -> String {
        guard let type, type.hasPrefix("claude_") else { return "" }
        var plan = type.dropFirst("claude_".count).capitalized
        if let multiplier = tier?.split(separator: "_").last, multiplier.hasSuffix("x"), multiplier.dropLast().allSatisfy(\.isNumber) {
            plan += " \(multiplier)"
        }
        return plan
    }
}
