import SwiftUI

struct DailyHeatmapView: View {
    let dailyTotals: [DailyTotal]
    let fromDate: String
    let toDate: String
    let mode: ThemeMode

    private let spacing: CGFloat = 2
    private let dayHeaders = ["日", "一", "二", "三", "四", "五", "六"]

    private let cells: [HeatCell]
    private let maxTokens: Int64
    private let dayCount: Int

    init(dailyTotals: [DailyTotal], fromDate: String, toDate: String, mode: ThemeMode) {
        self.dailyTotals = dailyTotals
        self.fromDate = fromDate
        self.toDate = toDate
        self.mode = mode

        let fmt = DateFormatter(); fmt.dateFormat = "yyyy-MM-dd"
        let cal = Calendar.current

        guard let from = fmt.date(from: fromDate),
              let to = fmt.date(from: toDate) else {
            cells = []; maxTokens = 0; dayCount = 0
            return
        }

        var lookup: [String: Int64] = [:]
        for d in dailyTotals { lookup[d.date] = d.tokens }

        var flat: [HeatCell] = []
        var maxT: Int64 = 0
        var cursor = from
        while cursor <= to {
            let key = fmt.string(from: cursor)
            let tokens = lookup[key] ?? 0
            let wd = cal.component(.weekday, from: cursor) - 1 // 0=Sun
            flat.append(HeatCell(date: key, tokens: tokens, weekday: wd))
            if tokens > maxT { maxT = tokens }
            cursor = cal.date(byAdding: .day, value: 1, to: cursor)!
        }
        cells = flat
        maxTokens = maxT
        dayCount = flat.count
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            headerBar
            cellStrip
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(Color.black.opacity(0.20))
                .overlay(
                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                        .strokeBorder(Color.white.opacity(0.06), lineWidth: 0.5)
                )
        )
    }

    private var headerBar: some View {
        HStack(spacing: 6) {
            Text(L10n.dailyTrend)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(Theme.tSecondary)
            Spacer()
            legendBar
        }
    }

    // MARK: - Cell strip (fills available width)

    private var cellStrip: some View {
        GeometryReader { geo in
            let totalSpacing = spacing * CGFloat(max(dayCount - 1, 0))
            let maxCell = CGFloat(18)
            let calc = (geo.size.width - totalSpacing) / CGFloat(max(dayCount, 1))
            let sz = min(maxCell, max(calc, 6))

            VStack(alignment: .leading, spacing: 2) {
                // Day-of-week labels
                HStack(spacing: spacing) {
                    ForEach(cells) { cell in
                        Text(dayHeaders[cell.weekday])
                            .font(.system(size: 7, design: .monospaced))
                            .foregroundStyle(Theme.tTertiary)
                            .frame(width: sz)
                    }
                }
                // Colored cells
                HStack(spacing: spacing) {
                    ForEach(cells) { cell in
                        RoundedRectangle(cornerRadius: 2)
                            .fill(color(cell.tokens))
                            .frame(width: sz, height: sz)
                    }
                }
            }
        }
        .frame(height: 32) // 2 rows of ~14px + spacing
    }

    // MARK: - Color

    private func color(_ tokens: Int64) -> Color {
        guard tokens > 0, maxTokens > 0 else { return Color.white.opacity(0.06) }
        let a = Theme.accent(mode)
        let ratio = Double(tokens) / Double(maxTokens)
        switch ratio {
        case 0.00..<0.15: return a.opacity(0.12)
        case 0.15..<0.35: return a.opacity(0.26)
        case 0.35..<0.60: return a.opacity(0.44)
        case 0.60..<0.85: return a.opacity(0.62)
        default:          return a.opacity(0.84)
        }
    }

    // MARK: - Legend

    private var legendBar: some View {
        HStack(spacing: 2) {
            Text(L10n.less).font(.system(size: 8)).foregroundStyle(Theme.tTertiary)
            ForEach(0..<5, id: \.self) { level in
                let ops: [Double] = [0.12, 0.26, 0.44, 0.62, 0.84]
                RoundedRectangle(cornerRadius: 1.5)
                    .fill(Theme.accent(mode).opacity(ops[level]))
                    .frame(width: 8, height: 8)
            }
            Text(L10n.more).font(.system(size: 8)).foregroundStyle(Theme.tTertiary)
        }
    }
}

private struct HeatCell: Identifiable {
    let id = UUID()
    let date: String
    let tokens: Int64
    let weekday: Int
}
