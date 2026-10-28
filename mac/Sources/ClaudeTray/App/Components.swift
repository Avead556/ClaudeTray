import SwiftUI


struct Avatar: View {
    let account: Account
    let isActive: Bool
    var size: CGFloat = 32

    static let palette: [(Color, Color)] = [
        (.indigo, .purple), (.blue, .cyan), (.orange, .pink), (.teal, .green), (.pink, .purple), (.cyan, .mint),
        (.red, .orange), (.yellow, .orange), (.gray, .secondary),
    ]

    static func colors(for account: Account) -> (Color, Color) {
        if let color = account.color, palette.indices.contains(color) { return palette[color] }
        return palette[account.id.unicodeScalars.reduce(0) { $0 + Int($1.value) } % 6]
    }

    var body: some View {
        let colors = Self.colors(for: account)
        Circle()
            .fill(LinearGradient(colors: [colors.0, colors.1], startPoint: .topLeading, endPoint: .bottomTrailing))
            .frame(width: size, height: size)
            .overlay {
                Text(initials).font(.system(size: size * 0.38, weight: .semibold)).foregroundStyle(.white)
            }
            .overlay(alignment: .bottomTrailing) {
                if isActive {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: size * 0.32, weight: .bold))
                        .symbolRenderingMode(.palette)
                        .foregroundStyle(.white, .green)
                        .background(Circle().fill(.background).padding(-1.5))
                        .offset(x: 1, y: 1)
                        .accessibilityLabel(Text("Active account"))
                }
            }
    }

    private var initials: String {
        let source = account.nickname ?? account.name
        let words = source.split(separator: " ")
        let letters = words.count >= 2 ? words.prefix(2).compactMap(\.first)
            : account.nickname != nil ? Array(source.prefix(2)) : Array(account.email.prefix(2))
        return String(letters).uppercased()
    }
}

func limitColor(_ limit: Usage.Limit) -> Color {
    if limit.isCritical { return .red }
    return switch limit.kind {
    case .session: .orange
    case .weekly: .indigo
    case .extra: .purple
    }
}

struct UsageRings: View {
    let usage: UsageMonitor.State?
    var size: CGFloat = 28

    var body: some View {
        HStack(spacing: 5) {
            switch usage {
            case .loaded(let usage):
                if let session = usage.session { ring(session, label: "\(session.percent)") }
                if let weekly = usage.weekly { ring(weekly, label: "\(weekly.percent)") }
                let extras = usage.extras
                if let top = extras.max(by: { $0.percent < $1.percent }) {
                    MiniRing(percent: top.percent, label: top.label, tint: limitColor(top), size: size)
                        .overlay(alignment: .topTrailing) {
                            if extras.count > 1 { CountBadge(count: extras.count - 1).offset(x: 4, y: -3) }
                        }
                        .help(extras.map(\.text).joined(separator: "\n"))
                }
            case .signedOut:
                status("exclamationmark.triangle.fill", tint: .orange, help: "Login expired — add the account again")
            case .unavailable:
                status("wifi.slash", tint: .secondary, help: "Limits unavailable")
            case nil:
                MiniRing(percent: 0, label: "", tint: .orange, size: size)
                MiniRing(percent: 0, label: "", tint: .indigo, size: size)
            }
        }
    }

    private func ring(_ limit: Usage.Limit, label: String) -> some View {
        MiniRing(percent: limit.percent, label: label, tint: limitColor(limit), size: size).help(limit.text)
    }

    private func status(_ symbol: String, tint: Color, help: LocalizedStringKey) -> some View {
        Image(systemName: symbol)
            .font(.system(size: 13))
            .foregroundStyle(tint)
            .frame(width: size, height: size)
            .help(Text(help))
    }
}

private struct CountBadge: View {
    let count: Int

    var body: some View {
        Text(verbatim: "+\(count)")
            .font(.system(size: 7, weight: .bold))
            .foregroundStyle(.white)
            .padding(.horizontal, 3)
            .padding(.vertical, 1)
            .background(Capsule().fill(.purple))
            .background(Capsule().fill(.background).padding(-1))
    }
}

struct MiniRing: View {
    let percent: Int
    let label: String
    let tint: Color
    var size: CGFloat = 28

    var body: some View {
        ZStack {
            Circle().stroke(Color.primary.opacity(0.08), lineWidth: 2.5)
            if percent > 0 {
                Circle()
                    .trim(from: 0, to: CGFloat(min(percent, 100)) / 100)
                    .stroke(tint, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
            Text(verbatim: label)
                .font(.system(size: size * (label.count > 2 ? 0.25 : 0.29), weight: .semibold))
                .foregroundStyle(percent > 0 ? AnyShapeStyle(tint) : AnyShapeStyle(.tertiary))
                .monospacedDigit()
        }
        .frame(width: size, height: size)
        .animation(.easeOut(duration: 0.4), value: percent)
    }
}

struct PlanBadge: View {
    let plan: String
    var fontSize: CGFloat = 9

    var body: some View {
        let style = Self.style(plan)
        Text(verbatim: style.title)
            .font(.system(size: fontSize, weight: .semibold))
            .tracking(0.3)
            .foregroundStyle(style.color)
            .padding(.horizontal, fontSize * 0.6)
            .padding(.vertical, fontSize * 0.15)
            .background(
                LinearGradient(colors: [style.color.opacity(0.18), style.glow.opacity(0.12)],
                               startPoint: .topLeading, endPoint: .bottomTrailing),
                in: RoundedRectangle(cornerRadius: 4)
            )
            .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(style.color.opacity(0.3), lineWidth: 0.5))
            .shadow(color: style.color.opacity(style.shines ? 0.25 : 0), radius: 4)
            .fixedSize()
    }

    private static func style(_ plan: String) -> (title: String, color: Color, glow: Color, shines: Bool) {
        if plan.hasPrefix("Max") {
            return ("MAX" + plan.dropFirst(3), .yellow, .orange, true)
        }
        if plan.hasPrefix("Team") || plan.hasPrefix("Enterprise") {
            return (plan, .cyan, .blue, true)
        }
        return (plan, .secondary, .secondary, false)
    }
}
