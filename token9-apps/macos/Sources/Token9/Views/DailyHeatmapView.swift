import SwiftUI

struct DailyHeatmapView: View {
    let dailyTotals: [DailyTotal]
    let mode: ThemeMode

    private let s: CGFloat = 3       // spacing
    private let c: CGFloat = 14      // cell
    private let cols = 7
    private let hdrs = ["日", "一", "二", "三", "四", "五", "六"]

    private let rows: [[HeatCell]]
    private let maxT: Int64
    private let label: String

    init(dailyTotals: [DailyTotal], mode: ThemeMode) {
        self.dailyTotals = dailyTotals
        self.mode = mode

        let fmt = DateFormatter(); fmt.dateFormat = "yyyy-MM-dd"
        let mf  = DateFormatter(); mf.dateFormat = "yyyy MMMM"
        let cal = Calendar.current; let now = Date()

        let comps = cal.dateComponents([.year, .month], from: now)
        guard let from = cal.date(from: comps),
              let dr = cal.range(of: .day, in: .month, for: now) else {
            rows = []; maxT = 0; label = ""
            return
        }
        let to = cal.date(byAdding: .day, value: dr.count - 1, to: from)!
        label = mf.string(from: from)

        let sun = cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: from))!
        let sat = cal.date(byAdding: .day, value: 6,
                           to: cal.date(from: cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: to))!)!

        var lu: [String: Int64] = [:]
        for d in dailyTotals { lu[d.date] = d.tokens }

        var flat: [HeatCell] = []
        var mx: Int64 = 0
        var cur = sun
        while cur <= sat {
            let k = fmt.string(from: cur); let v = lu[k] ?? 0
            let inR = cur >= from && cur <= to
            flat.append(HeatCell(date: k, tokens: v, inRange: inR))
            if inR, v > mx { mx = v }
            cur = cal.date(byAdding: .day, value: 1, to: cur)!
        }
        maxT = mx

        var r: [[HeatCell]] = []
        for i in stride(from: 0, to: flat.count, by: cols) {
            r.append(Array(flat[i..<min(i+cols, flat.count)]))
        }
        rows = r
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack(spacing: 0) {
                Text(label)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(Theme.tPrimary)
                Spacer()
                HStack(spacing: 2) {
                    Text(L10n.less).font(.system(size: 8)).foregroundStyle(Theme.tTertiary)
                    ForEach(0..<5, id: \.self) { lv in
                        RoundedRectangle(cornerRadius: 1.5)
                            .fill(fill([0.12, 0.26, 0.44, 0.62, 0.84][lv]))
                            .frame(width: 8, height: 8)
                    }
                    Text(L10n.more).font(.system(size: 8)).foregroundStyle(Theme.tTertiary)
                }
            }

            VStack(spacing: s) {
                HStack(spacing: s) {
                    ForEach(0..<cols, id: \.self) { i in
                        Text(hdrs[i]).font(.system(size: 8, design: .monospaced))
                            .foregroundStyle(Theme.tTertiary).frame(width: c)
                    }
                }
                ForEach(rows.indices, id: \.self) { ri in
                    HStack(spacing: s) {
                        ForEach(rows[ri]) { cl in
                            RoundedRectangle(cornerRadius: 2)
                                .fill(cl.inRange ? hex(cl.tokens) : Color.clear)
                                .frame(width: c, height: c)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .center)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(Color.black.opacity(0.14))
        )
    }

    private func hex(_ t: Int64) -> Color {
        let a = Theme.accent(mode)
        guard t > 0, maxT > 0 else { return Color.white.opacity(0.05) }
        switch Double(t) / Double(maxT) {
        case 0..<0.15: return a.opacity(0.12)
        case 0.15..<0.35: return a.opacity(0.26)
        case 0.35..<0.60: return a.opacity(0.44)
        case 0.60..<0.85: return a.opacity(0.62)
        default:        return a.opacity(0.84)
        }
    }

    private func fill(_ o: Double) -> Color {
        Theme.accent(mode).opacity(o)
    }
}

private struct HeatCell: Identifiable {
    let id = UUID()
    let date: String; let tokens: Int64; let inRange: Bool
}
