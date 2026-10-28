import AppKit

enum StatusIcon {
    private static let size = NSSize(width: 18, height: 18)
    private static let lineWidth: CGFloat = 1.8

    static func image(weekly: Usage.Limit?, session: Usage.Limit?) -> NSImage {
        let image = NSImage(size: size, flipped: false) { rect in
            let center = NSPoint(x: rect.midX, y: rect.midY)
            drawRing(center: center, radius: 7.9, limit: weekly)
            drawRing(center: center, radius: 4.9, limit: session)
            drawSpark(center: center, radius: 2.9)
            return true
        }
        let limits = [weekly, session].compactMap { $0 }
        image.accessibilityDescription = limits.isEmpty
            ? String(localized: "Claude Code accounts")
            : limits.map(\.text).joined(separator: ", ")
        return image
    }

    private static func drawRing(center: NSPoint, radius: CGFloat, limit: Usage.Limit?) {
        let track = NSBezierPath()
        track.appendArc(withCenter: center, radius: radius, startAngle: 0, endAngle: 360)
        track.lineWidth = lineWidth
        NSColor.labelColor.withAlphaComponent(0.3).setStroke()
        track.stroke()

        guard let limit, limit.percent > 0 else { return }
        let fraction = CGFloat(min(limit.percent, 100)) / 100
        let arc = NSBezierPath()
        arc.appendArc(withCenter: center, radius: radius, startAngle: 90, endAngle: 90 - 360 * fraction, clockwise: true)
        arc.lineWidth = lineWidth
        arc.lineCapStyle = .round
        (limit.isCritical ? NSColor.systemRed : NSColor.labelColor).setStroke()
        arc.stroke()
    }

    private static func drawSpark(center: NSPoint, radius: CGFloat) {
        NSColor.systemOrange.setStroke()
        for ray in 0..<12 {
            let angle = CGFloat(ray) / 12 * 2 * .pi + .pi / 2
            let length = ray.isMultiple(of: 2) ? radius : radius * 0.62
            let path = NSBezierPath()
            path.move(to: NSPoint(x: center.x + cos(angle) * radius * 0.18, y: center.y + sin(angle) * radius * 0.18))
            path.line(to: NSPoint(x: center.x + cos(angle) * length, y: center.y + sin(angle) * length))
            path.lineWidth = radius * 0.32
            path.lineCapStyle = .round
            path.stroke()
        }
    }
}
