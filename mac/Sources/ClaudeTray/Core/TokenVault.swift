import Foundation

enum TokenVault {
    private static let service = "ClaudeTray"

    static func token(for id: String) -> String? {
        Keychain.read(service: service, account: id)
    }

    static func save(_ oauth: String, for id: String) throws {
        try Keychain.write(service: service, account: id, secret: oauth)
    }

    static func delete(_ id: String) {
        Keychain.delete(service: service, account: id)
    }
}
