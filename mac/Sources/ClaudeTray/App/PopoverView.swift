import SwiftUI

struct PopoverActions {
    var switchTo: (Account) -> Void
    var restore: (Account) -> Void
    var add: () -> Void
    var settings: () -> Void
    var quit: () -> Void
    var resize: (CGFloat) -> Void = { _ in }
}

struct PopoverView: View {
    let store: AccountStore
    let usage: UsageMonitor
    let actions: PopoverActions
    var expandAll = false

    var body: some View {
        VStack(spacing: 0) {
            header
            if let issue = store.issue { IssueBanner(issue: issue, store: store, restore: actions.restore) }
            if store.accounts.isEmpty {
                Text("No accounts yet — sign in to Claude Code")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)
                    .padding(16)
                Divider().opacity(0.5)
            }
            ForEach(Array(store.accounts.enumerated()), id: \.element.id) { index, account in
                AccountRow(account: account, index: index, isActive: account.id == store.activeID,
                           usage: usage.states[account.id], expanded: expandAll) { actions.switchTo(account) }
                if account.id != store.accounts.last?.id { Divider().opacity(0.5) }
            }
        }
        .frame(width: 300)
        .fixedSize(horizontal: false, vertical: true)
        .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { actions.resize($0) }
    }

    private var header: some View {
        HStack {
            Text("Accounts").font(.system(size: 13, weight: .semibold))
            Spacer()
            HStack(spacing: 6) {
                headerButton("plus", label: "Add account", action: actions.add)
                    .keyboardShortcut("n")
                headerButton("gearshape", label: "Settings…", action: actions.settings)
                    .keyboardShortcut(",")
            }
        }
        .background {
            Button("Quit", action: actions.quit).keyboardShortcut("q").hidden()
        }
        .padding(.horizontal, 14)
        .padding(.top, 11)
        .padding(.bottom, 9)
        .overlay(alignment: .bottom) { Divider().opacity(0.5) }
    }

    private func headerButton(_ symbol: String, label: LocalizedStringKey, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 11, weight: .semibold))
                .frame(width: 24, height: 24)
                .background(.quaternary, in: Circle())
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .help(Text(label))
        .accessibilityLabel(Text(label))
    }
}


private struct AccountRow: View {
    let account: Account
    let index: Int
    let isActive: Bool
    let usage: UsageMonitor.State?
    @State var expanded: Bool
    let select: () -> Void
    @State private var hovered = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 6) {
                Button(action: select) {
                    HStack(spacing: 10) {
                        Avatar(account: account, isActive: isActive)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(account.title)
                                .font(.system(size: 12, weight: .medium))
                                .lineLimit(1)
                                .layoutPriority(1)
                            HStack(spacing: 5) {
                                let label = account.nickname != nil ? account.email
                                    : account.organization.isEmpty ? account.name : account.organization
                                if !label.isEmpty {
                                    Text(label).font(.system(size: 10)).foregroundStyle(.secondary).lineLimit(1)
                                }
                                if !account.plan.isEmpty { PlanBadge(plan: account.plan) }
                            }
                            .padding(.top, 1)
                        }
                        Spacer(minLength: 4)
                        UsageRings(usage: usage)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .numberShortcut(index)
                .help(isActive ? Text("Active account") : Text("Switch to this account"))

                Button {
                    expanded.toggle()
                } label: {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(expanded ? .secondary : .quaternary)
                        .rotationEffect(.degrees(expanded ? 90 : 0))
                        .animation(.easeOut(duration: 0.2), value: expanded)
                        .frame(width: 16, height: 28)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help(expanded ? Text("Hide limits") : Text("Show limits"))
            }
            if expanded {
                UsageDetails(usage: usage)
                    .padding(.top, 10)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(hovered && !isActive ? Color.primary.opacity(0.05) : .clear)
        .onHover { hovered = $0 }
        .clipped()
    }
}


private struct UsageDetails: View {
    let usage: UsageMonitor.State?

    var body: some View {
        VStack(spacing: 8) {
            switch usage {
            case .loaded(let usage):
                ForEach(Array(usage.limits.enumerated()), id: \.offset) { _, limit in row(limit) }
            case .signedOut:
                note("Login expired — add the account again")
            case .unavailable, nil:
                note("Limits unavailable")
            }
        }
        .padding(10)
        .background(Color.primary.opacity(0.04), in: RoundedRectangle(cornerRadius: 8))
    }

    private func row(_ limit: Usage.Limit) -> some View {
        VStack(spacing: 4) {
            HStack(spacing: 6) {
                Text(verbatim: limit.title).font(.system(size: 11))
                Spacer()
                if let reset = limit.resetText {
                    Text(verbatim: String(localized: "Resets \(reset)")).font(.system(size: 10)).foregroundStyle(.tertiary)
                }
                Text(verbatim: "\(limit.percent)%")
                    .font(.system(size: 11, weight: .semibold))
                    .monospacedDigit()
                    .foregroundStyle(limit.percent > 0 ? AnyShapeStyle(limitColor(limit)) : AnyShapeStyle(.tertiary))
                    .frame(minWidth: 30, alignment: .trailing)
            }
            GeometryReader { geometry in
                Capsule().fill(Color.primary.opacity(0.08))
                    .overlay(alignment: .leading) {
                        Capsule().fill(limitColor(limit))
                            .frame(width: geometry.size.width * CGFloat(min(limit.percent, 100)) / 100)
                    }
            }
            .frame(height: 3)
        }
    }

    private func note(_ text: LocalizedStringKey) -> some View {
        Text(text).font(.system(size: 11)).foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .leading)
    }
}


private extension View {
    @ViewBuilder
    func numberShortcut(_ index: Int) -> some View {
        if index < 9 { keyboardShortcut(KeyEquivalent(Character("\(index + 1)")), modifiers: .command) } else { self }
    }
}

private struct IssueBanner: View {
    let issue: AccountStore.Issue
    let store: AccountStore
    let restore: (Account) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label {
                Text(issue.message).font(.system(size: 11)).fixedSize(horizontal: false, vertical: true)
            } icon: {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
            }
            if case .tokenMismatch(let configured, _) = issue, store.accounts.contains(where: { $0.id == configured.id }) {
                Button(String(localized: "Restore \(configured.email)")) { restore(configured) }
                    .controlSize(.small)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
    }
}
