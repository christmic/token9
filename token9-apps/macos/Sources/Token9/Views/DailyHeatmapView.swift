import SwiftUI

struct DailyHeatmapView: View {
    let dailyTotals: [DailyTotal]
    let fromDate: String
    let toDate: String
    let mode: ThemeMode

    private let cols = 7
    private let spacing: CGFloat = 3
    private let cellSize: CGFloat = 16
    private let dayLabels = ["一", "二", "三", "四", "五", "六", "日"]

    private let rows: [[HeatCell]]
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
            rows = []; maxTokens = 0; dayCount = 0
            return
        }

        let dayDelta = cal.dateComponents([.day], from: from, to: to).day ?? 0
        dayCount = dayDelta + 1

        // Always pad to at least 2 full weeks.
        let startMonday = cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: from))!
        var endSunday = cal.date(byAdding: .day, value: 6,
                                 to: cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: to))!)!
        let minEnd = cal.date(byAdding: .day, value: 13, to: startMonday)! // at least 14 days
        if endSunday < minEnd { endSunday = minEnd }

        var lookup: [String: Int64] = [:]
        for d in dailyTotals { lookup[d.date] = d.tokens }

        var flat: [HeatCell] = []
        var maxT: Int64 = 0
        var cursor = startMonday
        while cursor <= endSunday {
            let key = fmt.string(from: cursor)
            let tokens = lookup[key] ?? 0
            let inRange = cursor >= from && cursor <= to
            flat.append(HeatCell(date: key, tokens: tokens, inRange: inRange))
            if inRange, tokens > maxT { maxT = tokens }
            cursor = cal.date(byAdding: .day, value: 1, to: cursor)!
        }
        maxTokens = maxT

        var r: [[HeatCell]] = []
        for i in stride(from: 0, to: flat.count, by: 7) {
            r.append(Array(flat[i..<min(i+7, flat.count)]))
        }
        rows = r
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            headerBar
            gridBody
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

    // MARK: - Header

    private var headerBar: some View {
        HStack(spacing: 6) {
            Text("每日趋势")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(Theme.tSecondary)
            Spacer()
            legendBar
        }
    }

    // MARK: - Grid

    private var gridBody: some View {
        HStack(alignment: .top, spacing: 3) {
            VStack(alignment: .trailing, spacing: spacing) {
                ForEach(0..<7, id: \.self) { i in
                    Text(dayLabels[i])
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(Theme.tTertiary)
                        .frame(height: cellSize)
                }
            }
            .frame(width: 14)

            VStack(alignment: .leading, spacing: spacing) {
                ForEach(rows.indices, id: \.self) { ri in
                    HStack(spacing: spacing) {
                        ForEach(rows[ri]) { cell in
                            RoundedRectangle(cornerRadius: 3)
                                .fill(cell.inRange ? heatmapColor(cell.tokens) : Color.white.opacity(0.03))
                                .frame(width: cellSize, height: cellSize)
                        }
                    }
                }
            }
        }
    }

    // MARK: - Color scale

    private func heatmapColor(_ tokens: Int64) -> Color {
        guard tokens > 0, maxTokens > 0 else {
            return Color.white.opacity(0.05)
        }
        let accent = Theme.accent(mode)
        let ratio = Double(tokens) / Double(maxTokens)
        switch ratio {
        case 0.00..<0.15: return accent.opacity(0.12)
        case 0.15..<0.35: return accent.opacity(0.26)
        case 0.35..<0.60: return accent.opacity(0.44)
        case 0.60..<0.85: return accent.opacity(0.62)
        default:          return accent.opacity(0.84)
        }
    }

    // MARK: - Legend

    private var legendBar: some View {
        HStack(spacing: 2) {
            Text("少").font(.system(size: 8)).foregroundStyle(Theme.tTertiary)
            ForEach(0..<5, id: \.self) { level in
                let ops: [Double] = [0.12, 0.26, 0.44, 0.62, 0.84]
                RoundedRectangle(cornerRadius: 1.5)
                    .fill(Theme.accent(mode).opacity(ops[level]))
                    .frame(width: 8, height: 8)
            }
            Text("多").font(.system(size: 8)).foregroundStyle(Theme.tTertiary)
        }
    }
}

// MARK: - Cell model

private struct HeatCell: Identifiable {
    let id = UUID()
    let date: String
    let tokens: Int64
    let inRange: Bool
}
