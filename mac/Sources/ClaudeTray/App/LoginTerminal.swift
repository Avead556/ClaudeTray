import AppKit

enum LoginTerminal {
    static func open() throws {
        let script = """
        #!/bin/zsh -l
        echo \(quoted(String(localized: "Sign in to the account you want to add.")))
        echo \(quoted(String(localized: "Then open the ClaudeTray menu — the account will appear in the list.")))
        echo
        claude auth login
        echo
        echo \(quoted(String(localized: "Done, you can close this window.")))
        """
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("claudetray-login.command")
        try script.write(to: url, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        NSWorkspace.shared.open(url)
    }

    private static func quoted(_ text: String) -> String {
        "'" + text.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
}
