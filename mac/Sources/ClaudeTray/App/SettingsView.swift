import ServiceManagement
import SwiftUI

struct SettingsView: View {
    let store: AccountStore
    let usage: UsageMonitor
    var quit: () -> Void = { NSApp.terminate(nil) }
    @State private var openAtLogin = SMAppService.mainApp.status == .enabled
    @State private var removing: Account?
    @State private var error: String?
    @AppStorage(Preferences.hotKeyEnabledKey) private var hotKeyEnabled = true
    @AppStorage(Preferences.hotKeyFailedKey) private var hotKeyFailed = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Accounts")
                .textCase(.uppercase)
                .font(.system(size: 11, weight: .semibold))
                .tracking(0.5)
                .foregroundStyle(.secondary)
                .padding(.bottom, 10)

            VStack(spacing: 8) {
                if store.accounts.isEmpty {
                    Text("No accounts yet — sign in to Claude Code")
                        .font(.system(size: 13))
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity)
                        .padding(14)
                        .background(card)
                }
                ForEach(store.accounts, id: \.id) { row($0) }
            }

            Divider().opacity(0.5).padding(.vertical, 14)

            HStack {
                Text("Open at login").font(.system(size: 13))
                Spacer()
                Toggle("Open at login", isOn: $openAtLogin)
                    .labelsHidden()
                    .toggleStyle(.switch)
                    .onChange(of: openAtLogin) { _, enabled in setOpenAtLogin(enabled) }
            }
            .padding(.horizontal, 2)


            HStack {
                Text("Switch to the next account").font(.system(size: 13))
                Spacer()
                ShortcutRecorder()
                    .disabled(!hotKeyEnabled)
                Toggle("Switch to the next account", isOn: $hotKeyEnabled)
                    .labelsHidden()
                    .toggleStyle(.switch)
            }
            .padding(.horizontal, 2)
            .padding(.top, 10)
            if hotKeyEnabled, hotKeyFailed {
                Label("This shortcut is taken by another app — choose another", systemImage: "exclamationmark.triangle.fill")
                    .font(.system(size: 11))
                    .foregroundStyle(.orange)
                    .padding(.horizontal, 2)
                    .padding(.top, 4)
            }

            HStack {
                Spacer()
                Button("Quit ClaudeTray", action: quit)
                    .buttonStyle(.link)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
            }
            .padding(.top, 14)
        }
        .padding(.horizontal, 20)
        .padding(.top, 20)
        .padding(.bottom, 16)
        .frame(width: 520)
        .fixedSize(horizontal: false, vertical: true)
        .confirmationDialog("Remove \(removing?.email ?? "")?", isPresented: .constant(removing != nil), presenting: removing) { account in
            Button("Remove", role: .destructive) {
                store.remove(account)
                usage.forget(account.id)
                removing = nil
            }
            Button("Cancel", role: .cancel) { removing = nil }
        } message: { _ in
            Text("The saved login will be removed from ClaudeTray. To use the account again, sign in to it again.")
        }
        .alert("ClaudeTray", isPresented: .constant(error != nil)) {
            Button("OK") { error = nil }
        } message: {
            Text(error ?? "")
        }
    }

    private var card: some View {
        RoundedRectangle(cornerRadius: 12)
            .fill(Color.primary.opacity(0.04))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Color.primary.opacity(0.06), lineWidth: 0.5))
    }

    private func row(_ account: Account) -> some View {
        let active = account.id == store.activeID
        return HStack(spacing: 10) {
            colorMenu(account) {
                Avatar(account: account, isActive: active, size: 34)
            }
            VStack(alignment: .leading, spacing: 1) {
                TextField(account.email, text: Binding(
                    get: { account.nickname ?? "" },
                    set: { store.setNickname($0, for: account.id) }
                ))
                .textFieldStyle(.plain)
                .font(.system(size: 13, weight: .medium))
                .help(Text("Account name — shown instead of the email"))
                let subtitle = [account.nickname != nil ? account.email : "", account.details].filter { !$0.isEmpty }
                if !subtitle.isEmpty {
                    Text(subtitle.joined(separator: " · ")).font(.system(size: 11)).foregroundStyle(.secondary).lineLimit(1)
                }
            }
            Spacer(minLength: 8)
            UsageRings(usage: usage.states[account.id], size: 30)
            HStack(spacing: 6) {
                if !account.plan.isEmpty { PlanBadge(plan: account.plan) }
                Button {
                    removing = account
                } label: {
                    Image(systemName: "trash")
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                        .frame(width: 28, height: 28)
                        .background(card.clipShape(RoundedRectangle(cornerRadius: 8)))
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled(active)
                .opacity(active ? 0.4 : 1)
                .help(active ? Text("You can’t remove the active account — switch to another one first") : Text("Remove account"))
                .accessibilityLabel(Text("Remove account"))
            }
        }
        .padding(14)
        .background(card)
    }

    private func colorMenu(_ account: Account, @ViewBuilder label: () -> some View) -> some View {
        Menu {
            ForEach(Avatar.palette.indices, id: \.self) { index in
                Toggle(isOn: Binding(get: { account.color == index }, set: { _ in store.setColor(index, for: account.id) })) {
                    Image(nsImage: Self.swatch(Avatar.palette[index]))
                }
            }
            Divider()
            Button("Default color") { store.setColor(nil, for: account.id) }
        } label: {
            label()
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .help(Text("Avatar color"))
    }

    private static func swatch(_ colors: (Color, Color)) -> NSImage {
        NSImage(size: NSSize(width: 14, height: 14), flipped: false) { rect in
            NSGradient(starting: NSColor(colors.0), ending: NSColor(colors.1))?
                .draw(in: NSBezierPath(ovalIn: rect.insetBy(dx: 1, dy: 1)), angle: -45)
            return true
        }
    }

    private func setOpenAtLogin(_ enabled: Bool) {
        do {
            if enabled { try SMAppService.mainApp.register() } else { try SMAppService.mainApp.unregister() }
        } catch {
            self.error = error.localizedDescription
            openAtLogin = SMAppService.mainApp.status == .enabled
        }
    }
}
