import AppKit

if CLI.requested || Snapshot.requested {
    Task { @MainActor in
        exit(CLI.requested ? await CLI.run() : await Snapshot.run())
    }
    RunLoop.main.run()
}

MainActor.assumeIsolated {
    let app = NSApplication.shared
    let delegate = AppDelegate()
    app.delegate = delegate
    app.setActivationPolicy(.accessory)
    app.run()
}
