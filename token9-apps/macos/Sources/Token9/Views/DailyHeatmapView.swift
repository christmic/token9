import SwiftUI

struct DailyHeatmapView: View {
    let dailyTotals: [DailyTotal]
    let fromDate: String
    let toDate: String
    let mode: ThemeMode

    private let spacing: CGFloat = 3
    private let cell: CGFloat = 14
    private let cols = 7
    private let headers = ["日", "一", "二", "三", "四", "五", "六"]

    private let rows: [[HeatCell]]
    private let maxTokens: Int64

    init(dailyTotals: [DailyTotal], fromDate: String, toDate: String, mode: ThemeMode) {
        self.dailyTotals = dailyTotals
        self.fromDate = fromDate
        self.toDate = toDate
        self.mode = mode

        let fmt = DateFormatter(); fmt.dateFormat = "yyyy-MM-dd"
        let cal = Calendar.current
        var sunFirst = cal; sunFirst.firstWeekday = 1

        guard let from = fmt.date(from: fromDate),
              let to = fmt.date(from: toDate) else {
            rows = []; maxTokens = 0
            return
        }

        // Pad to full weeks around the range, min 4 weeks (28 days).
        let fromSunday = sunFirst.date(from: sunFirst.dateComponents([.yearForWeekOfYear, .weekOfYear], from: from))!
        var toSaturday = sunFirst.date(byAdding: .day, value: 6,
                                       to: sunFirst.date(from: sunFirst.dateComponents([.yearForWeekOfYear, .weekOfYear], from: to))!)!
        let minSat = sunFirst.date(byAdding: .day, value: 27, to: fromSunday)!
        if toSaturday < minSat { toSaturday = minSat }

        var lookup: [String: Int64] = [:]
        for d in dailyTotals { lookup[d.date] = d.tokens }

        var maxT: Int64 = 0
        var flat: [HeatCell] = []
        var cursor = fromSunday
        while cursor <= toSaturday {
            let key = fmt.string(from: cursor)
            let tokens = lookup[key] ?? 0
            let inRange = cursor >= from && cursor <= to
            let wd = sunFirst.component(.weekday, from: cursor) - 1
            flat.append(HeatCell(date: key, tokens: tokens, inRange: inRange, weekday: wd))
            if inRange, tokens > maxT { maxT = tokens }
            cursor = sunFirst.date(byAdding: .day, value: 1, to: cursor)!
        }
        maxTokens = maxT

        var r: [[HeatCell]] = []
        for i in stride(from: 0, to: flat.count, by: cols) {
            r.append(Array(flat[i..<min(i+cols, flat.count)]))
        }
        rows = r
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
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
            Text(L10n.dailyTrend)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(Theme.tSecondary)
            Spacer()
            legendBar
        }
    }

    // MARK: - Grid

    private var grid: some View {
        let colHeaderW: CGFloat = 16

        return VStack(spacing: 0) {
            // Column headers
            HStack(spacing: 0) {
                Spacer().frame(width: colHeaderW)
                HStack(spacing: spacing) {
                    ForEach(0..<cols, id: \.self) { i in
                        Text(headers[i])
                            .font(.system(size: 8, design: .monospaced))
                            .foregroundStyle(Theme.tTertiary)
                            .frame(width: cell)
                    }
                }
                Spacer(minLength: 0)
            }

            // Rows with week-number labels
            ForEach(rows.indices, id: \.self) { ri in
                HStack(spacing: 0) {
                    Text("\(ri + 1)")
                        .font(.system(size: 8, design: .monospaced))
                        .foregroundStyle(Theme.tTertiary)
                        .frame(width: colHeaderW, alignment: .trailing)
                    HStack(spacing: spacing) {
                        ForEach(rows[ri]) { c in
                            RoundedRectangle(cornerRadius: 2)
                                .fill(c.inRange ? color(c.tokens) : Color.white.opacity(0.03))
                                .frame(width: cell, height: cell)
                        }
                    }
                    Spacer(minLength: 0)
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
