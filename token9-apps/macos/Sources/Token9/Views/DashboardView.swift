import SwiftUI

struct DashboardView: View {
    @StateObject private var vm = DashboardViewModel()
    @AppStorage("themeMode") private var themeRaw = ThemeMode.warm.rawValue
    @AppStorage("lang") private var langRaw = AppLang.zh.rawValue

    private var mode: ThemeMode { ThemeMode(rawValue: themeRaw) ?? .warm }

    var body: some View {
        ZStack {
            VisualEffect().ignoresSafeArea()
            Theme.bg(mode).ignoresSafeArea()

            VStack(alignment: .leading, spacing: 0) {
                topBar
                SegmentedTabs(sel: $vm.range)
                groupBar
                ScrollView {
                    VStack(alignment: .leading, spacing: 8) {
                        heatmapSection
                        contentBody
                    }
                    .padding(.bottom, 6)
                }
            }
            .padding(Theme.outerPad)
        }
        .frame(width: 500, height: 660)
        .onAppear { vm.start() }
        .onDisappear { vm.stop() }
    }

    private var topBar: some View {
        HStack(alignment: .center, spacing: 9) {
            Image(systemName: "timer")
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(Theme.accent(mode))
            Text("token9").font(.system(size: 15, weight: .bold)).foregroundStyle(Theme.tPrimary)
            Spacer()
            if let at = vm.updatedAt {
                Text(timeString(at)).font(.system(size: 9.5, design: .monospaced))
                    .foregroundStyle(Theme.tTertiary)
            }
            IconButton(icon: mode.icon) {
                withAnimation(.easeOut(duration: 0.25)) { themeRaw = mode.next.rawValue }
            }
            Button {
                let cur = AppLang(rawValue: langRaw) ?? .zh
                withAnimation(.easeOut(duration: 0.25)) { langRaw = cur.next.rawValue }
            } label: {
                Text(AppLang(rawValue: langRaw)?.label ?? "中")
                    .font(.system(size: 10, weight: .semibold, design: .monospaced))
                    .foregroundStyle(Theme.tSecondary)
                    .frame(width: 28, height: 24)
                    .background(RoundedRectangle(cornerRadius: 7, style: .continuous)
                        .fill(Color.primary.opacity(0.08)))
            }
            .buttonStyle(.plain)
            IconButton(icon: "arrow.clockwise") { vm.reload() }
        }
    }

    private var groupBar: some View {
        Picker("", selection: $vm.groupBy) {
            ForEach(GroupBy.allCases) { g in Text(g.label).tag(g) }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .frame(width: 150)
    }

    @ViewBuilder
    private var heatmapSection: some View {
        if vm.range == .today, !vm.dailyTotals.isEmpty {
            DailyHeatmapView(
                dailyTotals: vm.dailyTotals,
                mode: mode
            )
            .transition(.opacity)
        }
    }

    @ViewBuilder
    private var contentBody: some View {
        if let e = vm.error, vm.cards.isEmpty {
            errorState(e)
        } else if vm.cards.isEmpty && !vm.loading {
            emptyState
        } else {
            VStack(alignment: .leading, spacing: 13) {
                ForEach(vm.cards) { card in
                    GroupCardView(card: card, subTitle: vm.groupBy.subTitle)
                }
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 8) {
            Image(systemName: "chart.bar.doc.horizontal")
                .font(.system(size: 30)).foregroundStyle(Theme.tTertiary)
            Text(L10n.noData).font(.system(size: 13, weight: .medium)).foregroundStyle(Theme.tSecondary)
            Text(L10n.noDataHint)
                .font(.system(size: 10)).foregroundStyle(Theme.tTertiary).multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorState(_ e: String) -> some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 26)).foregroundStyle(.orange)
            Text(L10n.connFailed).font(.system(size: 13, weight: .medium)).foregroundStyle(Theme.tSecondary)
            Text(L10n.connHint).font(.system(size: 10)).foregroundStyle(Theme.tTertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func timeString(_ d: Date) -> String {
        let f = DateFormatter(); f.dateFormat = "HH:mm:ss"; return f.string(from: d)
    }
}
