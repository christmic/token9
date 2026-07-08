import SwiftUI

struct DailyHeatmapView: View {
    let dailyTotals: [DailyTotal]
    let mode: ThemeMode

    private let spacing: CGFloat = 3
    private let cell: CGFloat = 14
    private let cols = 7
    private let headers = ["日", "一", "二", "三", "四", "五", "六"]

    private let rows: [[HeatCell]]
    private let maxTokens: Int64
    private let monthLabel: String

    init(dailyTotals: [DailyTotal], mode: ThemeMode) {
        self.dailyTotals = dailyTotals
        self.mode = mode

        let fmt = DateFormatter(); fmt.dateFormat = "yyyy-MM-dd"
        let monthFmt = DateFormatter(); monthFmt.dateFormat = "yyyy MMMM"
        let cal = Calendar.current
        let now = Date()

        // Full current month: 1st … last day.
        let comps = cal.dateComponents([.year, .month], from: now)
        guard let from = cal.date(from: comps),
              let dayRange = cal.range(of: .day, in: .month, for: now) else {
            rows = []; maxTokens = 0; monthLabel = ""
            return
        }
        let to = cal.date(byAdding: .day, value: dayRange.count - 1, to: from)!
        monthLabel = monthFmt.string(from: from)

        // Extend from to full weeks (Sunday of from-week … Saturday of to-week).
        let fromSunday = cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: from))!
        let toSaturday = cal.date(byAdding: .day, value: 6,
                                  to: cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: to))!)!

        var lookup: [String: Int64] = [:]
        for d in dailyTotals { lookup[d.date] = d.tokens }

        var flat: [HeatCell] = []
        var maxT: Int64 = 0
        var cursor = fromSunday
        while cursor <= toSaturday {
            let key = fmt.string(from: cursor)
            let tokens = lookup[key] ?? 0
            let inRange = cursor >= from && cursor <= to
            let wd = cal.component(.weekday, from: cursor) - 1
            flat.append(HeatCell(date: key, tokens: tokens, inRange: inRange, weekday: wd))
            if inRange, tokens > maxT { maxT = tokens }
            cursor = cal.date(byAdding: .day, value: 1, to: cursor)!
        }
        maxTokens = maxT

        var r: [[HeatCell]] = []
        for i in stride(from: 0, to: flat.count, by: cols) {
            r.append(Array(flat[i..<min(i+cols, flat.count)]))
        }
        rows = r
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            headerBar
            grid
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
            Text(monthLabel)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(Theme.tSecondary)
            Spacer()
            legendBar
        }
    }

    // MARK: - Grid

    private var grid: some View {
        VStack(spacing: spacing) {
            // Day-of-week headers
            HStack(spacing: spacing) {
                ForEach(0..<cols, id: \.self) { i in
                    Text(headers[i])
                        .font(.system(size: 8, design: .monospaced))
                        .foregroundStyle(Theme.tTertiary)
                        .frame(width: cell)
                }
            }

            // Rows
            ForEach(rows.indices, id: \.self) { ri in
                HStack(spacing: spacing) {
                    ForEach(rows[ri]) { c in
                        RoundedRectangle(cornerRadius: 2)
                            .fill(c.inRange ? color(c.tokens) : Color.clear)
                            .frame(width: cell, height: cell)
                    }
                }
            }
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Color

    private func color(_ tokens: Int64) -> Color {
        guard tokens > 0, maxTokens > 0 else { return Color.white.opacity(0.05) }
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
    let inRange: Bool
    let weekday: Int
}
