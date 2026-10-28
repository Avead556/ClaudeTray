import AppKit
import Carbon.HIToolbox

struct Shortcut: Equatable {
    var keyCode: UInt32
    var modifiers: UInt32
    var title: String

    static let defaultNext = Shortcut(keyCode: UInt32(kVK_ANSI_A), modifiers: UInt32(controlKey | optionKey | cmdKey), title: "⌃⌥⌘A")

    private static let codeKey = "nextAccountShortcutCode"
    private static let modifiersKey = "nextAccountShortcutModifiers"
    private static let titleKey = "nextAccountShortcutTitle"

    static var saved: Shortcut {
        let defaults = UserDefaults.standard
        guard let title = defaults.string(forKey: titleKey) else { return defaultNext }
        return Shortcut(keyCode: UInt32(defaults.integer(forKey: codeKey)),
                        modifiers: UInt32(defaults.integer(forKey: modifiersKey)), title: title)
    }

    func save() {
        let defaults = UserDefaults.standard
        defaults.set(Int(keyCode), forKey: Self.codeKey)
        defaults.set(Int(modifiers), forKey: Self.modifiersKey)
        defaults.set(title, forKey: Self.titleKey)
    }

    init?(event: NSEvent) {
        let flags = event.modifierFlags.intersection([.control, .option, .shift, .command])
        guard !flags.intersection([.control, .option, .command]).isEmpty,
              let key = event.charactersIgnoringModifiers?.uppercased(), !key.isEmpty else { return nil }
        var modifiers = 0
        var title = ""
        if flags.contains(.control) { modifiers |= controlKey; title += "⌃" }
        if flags.contains(.option) { modifiers |= optionKey; title += "⌥" }
        if flags.contains(.shift) { modifiers |= shiftKey; title += "⇧" }
        if flags.contains(.command) { modifiers |= cmdKey; title += "⌘" }
        self.init(keyCode: UInt32(event.keyCode), modifiers: UInt32(modifiers), title: title + Self.keyName(event.keyCode, fallback: key))
    }

    init(keyCode: UInt32, modifiers: UInt32, title: String) {
        self.keyCode = keyCode
        self.modifiers = modifiers
        self.title = title
    }

    private static func keyName(_ code: UInt16, fallback: String) -> String {
        switch Int(code) {
        case kVK_Space: "Space"
        case kVK_Return: "↩"
        case kVK_Tab: "⇥"
        case kVK_Delete: "⌫"
        case kVK_LeftArrow: "←"
        case kVK_RightArrow: "→"
        case kVK_UpArrow: "↑"
        case kVK_DownArrow: "↓"
        default: fallback
        }
    }
}

@MainActor
final class HotKey {
    private var hotKey: EventHotKeyRef?
    private var handler: EventHandlerRef?
    private let action: () -> Void
    private(set) var shortcut: Shortcut?
    private(set) var failed = false

    init(action: @escaping () -> Void) {
        self.action = action
        var spec = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        InstallEventHandler(GetApplicationEventTarget(), { _, _, userData in
            guard let userData else { return noErr }
            let hotKey = Unmanaged<HotKey>.fromOpaque(userData).takeUnretainedValue()
            MainActor.assumeIsolated { hotKey.action() }
            return noErr
        }, 1, &spec, Unmanaged.passUnretained(self).toOpaque(), &handler)
    }

    @discardableResult
    func set(_ shortcut: Shortcut?) -> Bool {
        if let hotKey { UnregisterEventHotKey(hotKey) }
        hotKey = nil
        self.shortcut = shortcut
        failed = false
        guard let shortcut else { return true }
        let status = RegisterEventHotKey(shortcut.keyCode, shortcut.modifiers, EventHotKeyID(signature: 0x434C_5459, id: 1),
                                         GetApplicationEventTarget(), 0, &hotKey)
        failed = status != noErr
        return !failed
    }
}
