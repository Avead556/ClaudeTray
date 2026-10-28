import Foundation

enum AppPaths {
    static let directory: URL = {
        let directory = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("ClaudeTray", isDirectory: true)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }()

    static let accounts = directory.appendingPathComponent("accounts.json")
    static let usage = directory.appendingPathComponent("usage.json")
}
