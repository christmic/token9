import SwiftUI

/// Supported languages for dynamic UI switching.
enum AppLang: String, CaseIterable, Identifiable {
    case zh, en
    var id: String { rawValue }
    var label: String { self == .zh ? "中" : "EN" }
    var next: AppLang { self == .zh ? .en : .zh }
}

/// Lightweight localisation — reads @AppStorage("lang") so views re-render on switch.
enum L10n {
    private static var lang: AppLang {
        AppLang(rawValue: UserDefaults.standard.string(forKey: "lang") ?? "zh") ?? .zh
    }

    // MARK: - Time ranges
    static var today: String     { lang == .zh ? "今日" : "Today" }
    static var yesterday: String { lang == .zh ? "昨日" : "Yesterday" }
    static var week: String      { lang == .zh ? "本周" : "This Week" }
    static var lastWeek: String  { lang == .zh ? "上周" : "Last Week" }
    static var month: String     { lang == .zh ? "本月" : "This Month" }
    static var year: String      { lang == .zh ? "本年" : "This Year" }

    // MARK: - Grouping
    static var groupByTool: String  { lang == .zh ? "工具" : "Tool" }
    static var groupByModel: String { lang == .zh ? "模型" : "Model" }
    static var subByModel: String   { lang == .zh ? "按模型" : "By Model" }
    static var subByTool: String    { lang == .zh ? "按工具" : "By Tool" }

    // MARK: - Dashboard
    static var dailyTrend: String   { lang == .zh ? "每日趋势" : "Daily" }
    static var less: String         { lang == .zh ? "少" : "Less" }
    static var more: String         { lang == .zh ? "多" : "More" }
    static var noData: String       { lang == .zh ? "暂无数据" : "No data yet" }
    static var noDataHint: String   { lang == .zh ? "让工具走 token9 (127.0.0.1:9527) 后即可看到用量" : "Route your tools through token9 (127.0.0.1:9527)" }
    static var connFailed: String   { lang == .zh ? "连接失败" : "Connection failed" }
    static var connHint: String     { lang == .zh ? "token9 serve 在 :9527 运行吗？" : "Is token9 serve running on :9527?" }
    static var requests: String     { lang == .zh ? "请求" : "Requests" }
    static var input: String        { lang == .zh ? "输入" : "Input" }
    static var output: String       { lang == .zh ? "输出" : "Output" }
    static var cacheRead: String    { lang == .zh ? "缓存读取" : "Cache read" }
    static var cacheWrite: String   { lang == .zh ? "缓存写入" : "Cache write" }
    static var cacheHit: String     { lang == .zh ? "缓存命中" : "Cache hit" }
    static var remaining: String    { lang == .zh ? "剩余" : "Remaining" }
    static var limit: String        { lang == .zh ? "限制" : "Limit" }
    static var total: String        { lang == .zh ? "总计" : "Total" }

    // MARK: - Day labels
    static func dayLabel(_ idx: Int) -> String {
        let zh = ["一", "二", "三", "四", "五", "六", "日"]
        let en = ["M", "T", "W", "T", "F", "S", "S"]
        return lang == .zh ? zh[idx] : en[idx]
    }
}
