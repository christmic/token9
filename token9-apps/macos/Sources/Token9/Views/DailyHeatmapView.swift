import SwiftUI

// MARK: - GitHub-style daily consumption heatmap

struct DailyHeatmapView: View {
    let dailyTotals: [DailyTotal]
    let fromDate: String
    let toDate: String
    let mode: ThemeMode

    private let columns = 7
    private let cellSpacing: CGFloat = 3
    private let dayLabels = ["一", "二", "三", "四", "五", "六", "日"]

    private let cells: [HeatmapCell]
    private let rows: [[HeatmapCell]]
    private let maxTokens: Int64

    init(dailyTotals: [DailyTotal], fromDate: String, toDate: String, mode: ThemeMode) {
        self.dailyTotals = dailyTotals
        self.fromDate = fromDate
        self.toDate = toDate
        self.mode = mode

        let fmt = DateFormatter(); fmt.dateFormat = "yyyy-MM-dd"
        let cal = Calendar.current

        guard let from = fmt.date(from: fromDate),
              let to = fmt.date(from: toDate) else {
            cells = []; rows = []; maxTokens = 0
            return
        }

        let monday = cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: from))!
        let endSunday = cal.date(byAdding: .day, value: 6,
                                 to: cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: to))!)!

        var lookup: [String: Int64] = [:]
        for d in dailyTotals { lookup[d.date] = d.tokens }

        var flat: [HeatmapCell] = []
        var maxT: Int64 = 0
        var cursor = monday
        while cursor <= endSunday {
            let key = fmt.string(from: cursor)
            let tokens = lookup[key] ?? 0
            let inRange = cursor >= from && cursor <= to
            flat.append(HeatmapCell(date: key, tokens: tokens, inRange: inRange))
            if tokens > maxT { maxT = tokens }
            cursor = cal.date(byAdding: .day, value: 1, to: cursor)!
        }
        cells = flat
        maxTokens = maxT

        var r: [[HeatmapCell]] = []
        for i in stride(from: 0, to: flat.count, by: 7) {
            r.append(Array(flat[i..<min(i+7, flat.count)]))
        }
        rows = r
    }

    var body: some View {
        Card(tint: Theme.accent(mode).opacity(0.3)) {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("每日趋势").font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(Theme.tSecondary)
                    Spacer()
                    Text("\(dailyTotals.count) 天")
                        .font(.system(size: 10, weight: .medium, design: .monospaced))
                        .foregroundStyle(Theme.tTertiary)
                }

                gridBody

                legendBar
            }
        }
    }

    // MARK: - Grid

    private var gridBody: some View {
        HStack(alignment: .top, spacing: 4) {
            VStack(alignment: .trailing, spacing: cellSpacing) {
                ForEach(0..<7, id: \.self) { i in
                    Text(dayLabels[i])
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(Theme.tTertiary)
                        .frame(height: cellSize)
                }
            }
            .frame(width: 14)

            VStack(alignment: .leading, spacing: cellSpacing) {
                ForEach(rows.indices, id: \.self) { rowIdx in
                    HStack(spacing: cellSpacing) {
                        ForEach(rows[rowIdx]) { cell in
                            RoundedRectangle(cornerRadius: 3)
                                .fill(cell.inRange ? heatmapColor(cell.tokens) : Color.clear)
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
        case 0.15..<0.35: return accent.opacity(0.24)
        case 0.35..<0.60: return accent.opacity(0.40)
        case 0.60..<0.85: return accent.opacity(0.58)
        default:          return accent.opacity(0.80)
        }
    }

    // MARK: - Legend

    private var legendBar: some View {
        HStack(spacing: 4) {
            Spacer()
            Text("少").font(.system(size: 9)).foregroundStyle(Theme.tTertiary)
            ForEach(0..<5, id: \.self) { level in
                let opacities: [Double] = [0.12, 0.24, 0.40, 0.58, 0.80]
                RoundedRectangle(cornerRadius: 2)
                    .fill(Theme.accent(mode).opacity(opacities[level]))
                    .frame(width: 10, height: 10)
            }
            Text("多").font(.system(size: 9)).foregroundStyle(Theme.tTertiary)
        }
    }

    // MARK: - Sizing

    private var cellSize: CGFloat {
        let rowCount = rows.count
        if rowCount <= 1 { return 26 }
        if rowCount <= 5 { return 16 }
        return 12
    }
}

// MARK: - Cell model

private struct HeatmapCell: Identifiable {
    let id = UUID()
    let date: String
    let tokens: Int64
    let inRange: Bool
}
