import Foundation

struct Usage {
    struct Limit {
        enum Kind { case session, weekly, extra }

        var kind: Kind
        var name = ""
        var percent: Int
        var resetsAt: Date?
        var weeklyScope = false
        var label = ""
        var critical = false

        var isCritical: Bool { percent >= 90 || critical }

        var title: String {
            switch kind {
            case .session: String(localized: "Session")
            case .weekly: String(localized: "Week")
            case .extra: weeklyScope ? String(localized: "\(name) (week)") : name
            }
        }

        var resetText: String? {
            guard let resetsAt else { return nil }
            return kind == .session
                ? resetsAt.formatted(.relative(presentation: .named, unitsStyle: .abbreviated))
                : resetsAt.formatted(.dateTime.weekday(.abbreviated).hour().minute())
        }

        var text: String {
            let percent = "\(percent)%"
            switch (kind, resetText) {
            case (.session, let reset?): return String(localized: "Session: \(percent) · resets \(reset)")
            case (.session, nil): return String(localized: "Session: \(percent)")
            case (.weekly, let reset?): return String(localized: "Week: \(percent) · resets \(reset)")
            case (.weekly, nil): return String(localized: "Week: \(percent)")
            case (.extra, _): return weeklyScope ? String(localized: "\(name) (week): \(percent)") : "\(name): \(percent)"
            }
        }
    }

    var limits: [Limit] = []

    var session: Limit? { limits.first { $0.kind == .session } }
    var weekly: Limit? { limits.first { $0.kind == .weekly } }
    var extras: [Limit] { limits.filter { $0.kind == .extra } }

    var entries: [(text: String, percent: Int)] { limits.map { ($0.text, $0.percent) } }
    var lines: [String] { entries.map(\.text) }

    init(json: [String: Any]) {
        for item in json["limits"] as? [[String: Any]] ?? [] {
            guard let percent = item["percent"] as? Double else { continue }
            var limit = Limit(kind: .extra, percent: Int(percent.rounded()), resetsAt: Self.parseDate(item["resets_at"]))
            limit.critical = item["severity"] as? String == "critical"
            let kind = item["kind"] as? String ?? ""
            let weekly = item["group"] as? String == "weekly"
            let model = ((item["scope"] as? [String: Any])?["model"] as? [String: Any])?["display_name"] as? String
            if let model {
                limit.name = model
                limit.weeklyScope = weekly
            } else if kind == "session", session == nil {
                limit.kind = .session
            } else if weekly, self.weekly == nil {
                limit.kind = .weekly
            } else {
                limit.name = kind.split(separator: "_").first.map { $0.capitalized } ?? kind
            }
            limits.append(limit)
        }
        if limits.isEmpty {
            if let session = Self.window(json["five_hour"]) { limits.append(Limit(kind: .session, percent: session.0, resetsAt: session.1)) }
            if let weekly = Self.window(json["seven_day"]) { limits.append(Limit(kind: .weekly, percent: weekly.0, resetsAt: weekly.1)) }
        }
        assignLabels()
    }

    private mutating func assignLabels() {
        let names = limits.filter { $0.kind == .extra }.map { Array($0.name) }
        for index in limits.indices where limits[index].kind == .extra {
            let name = Array(limits[index].name)
            guard let first = name.first else { continue }
            let rivals = names.filter { $0 != name && $0.first?.lowercased() == first.lowercased() }
            var label = String(first).uppercased()
            if !rivals.isEmpty, name.count > 1 {
                let differs = (1..<name.count).first { i in
                    rivals.allSatisfy { i >= $0.count || $0[i].lowercased() != name[i].lowercased() }
                }
                label += String(name[differs ?? name.count - 1]).lowercased()
            }
            limits[index].label = label
        }
    }

    private static func window(_ value: Any?) -> (Int, Date?)? {
        guard let dict = value as? [String: Any], let utilization = dict["utilization"] as? Double else { return nil }
        return (Int(utilization.rounded()), parseDate(dict["resets_at"]))
    }

    private static func parseDate(_ value: Any?) -> Date? {
        guard let string = value as? String else { return nil }
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = formatter.date(from: string) { return date }
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.date(from: string)
    }
}
