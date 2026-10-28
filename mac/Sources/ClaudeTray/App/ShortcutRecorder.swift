import AppKit
import SwiftUI

struct ShortcutRecorder: View {
    @AppStorage("nextAccountShortcutTitle") private var title = Shortcut.defaultNext.title
    @State private var recording = false
    @State private var monitor: Any?

    var body: some View {
        Button {
            recording ? stop() : start()
        } label: {
            Text(recording ? String(localized: "Press a shortcut…") : title)
                .font(.system(size: 12, weight: .medium).monospaced())
                .frame(minWidth: 80)
        }
        .help(Text("Click to change the shortcut"))
        .onDisappear(perform: stop)
    }

    private func start() {
        recording = true
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            if event.keyCode == 53 {
                MainActor.assumeIsolated { stop() }
                return nil
            }
            guard let shortcut = Shortcut(event: event) else { return nil }
            MainActor.assumeIsolated {
                shortcut.save()
                stop()
            }
            return nil
        }
    }

    private func stop() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
        recording = false
    }
}
