import AppKit
import SwiftUI

@MainActor
final class PopoverController: NSObject, NSPopoverDelegate {
    private static let width: CGFloat = 300

    private let popover = NSPopover()
    private var host: NSHostingController<PopoverView>!
    private var appearing = false
    private var pendingHeight: CGFloat?

    var isShown: Bool { popover.isShown }

    init(store: AccountStore, usage: UsageMonitor, actions: PopoverActions) {
        super.init()
        var actions = actions
        actions.resize = { [weak self] in self?.resize(to: $0) }
        host = NSHostingController(rootView: PopoverView(store: store, usage: usage, actions: actions))
        host.sizingOptions = [.intrinsicContentSize]

        let container = NSViewController()
        container.view = NSView()
        container.addChild(host)
        container.view.addSubview(host.view)
        host.view.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            host.view.topAnchor.constraint(equalTo: container.view.topAnchor),
            host.view.leadingAnchor.constraint(equalTo: container.view.leadingAnchor),
            host.view.trailingAnchor.constraint(equalTo: container.view.trailingAnchor),
        ])
        popover.contentViewController = container
        popover.delegate = self
        popover.behavior = .transient
        popover.animates = true
    }

    func show(below button: NSStatusBarButton) {
        host.view.layoutSubtreeIfNeeded()
        popover.contentSize = NSSize(width: Self.width, height: ceil(host.view.fittingSize.height))
        appearing = true
        popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
        popover.contentViewController?.view.window?.makeKey()
    }

    func close() {
        popover.performClose(nil)
    }

    func popoverDidShow(_ notification: Notification) {
        appearing = false
        if let height = pendingHeight {
            pendingHeight = nil
            resize(to: height)
        }
    }

    func popoverDidClose(_ notification: Notification) {
        appearing = false
    }

    private func resize(to height: CGFloat) {
        guard !appearing else {
            pendingHeight = height
            return
        }
        let size = NSSize(width: Self.width, height: ceil(height))
        if popover.contentSize != size { popover.contentSize = size }
    }
}
