import Foundation

enum Preferences {
    static let hotKeyEnabledKey = "nextAccountHotKey"
    static let hotKeyFailedKey = "nextAccountShortcutFailed"

    static var hotKeyEnabled: Bool {
        UserDefaults.standard.object(forKey: hotKeyEnabledKey) as? Bool ?? true
    }

    static var hotKeyFailed: Bool {
        get { UserDefaults.standard.bool(forKey: hotKeyFailedKey) }
        set {
            guard newValue != hotKeyFailed else { return }
            UserDefaults.standard.set(newValue, forKey: hotKeyFailedKey)
        }
    }
}

extension Notification.Name {
    static let accountsChanged = Notification.Name("local.claudetray.changed")
}
