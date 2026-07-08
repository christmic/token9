import Charts
import SwiftUI

@MainActor
final class StatsViewModel: ObservableObject {
    @Published var buckets: [StatBucketDto] = []
    @Published var error: String?
    @Published var loading = false

    private let client = Token9Client()

    func load() async {
        loading = true
        error = nil
        do {
            buckets = try await client.stats().buckets
        } catch {
            self.error = error.localizedDescription
        }
        loading = false
    }
}

struct StatsView: View {
    @StateObject private var vm = StatsViewModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            header

            if vm.loading {
                ProgressView().frame(maxWidth: .infinity)
            }

            if let error = vm.error {
                VStack(alignment: .leading, spacing: 4) {
                    Text(error).foregroundStyle(.red)
                    Text("Is `token9 serve` running on :9527?")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }

            if !vm.buckets.isEmpty {
                chart
                table
            } else if !vm.loading && vm.error == nil {
                Text("No data yet — send some traffic through token9.")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }

            Spacer(minLength: 0)
        }
        .padding()
        .task { await vm.load() }
    }

    private var header: some View {
        HStack {
            Text("token9 usage").font(.title2).bold()
            Spacer()
            Button {
                Task { await vm.load() }
            } label: {
                Label("Refresh", systemImage: "arrow.clockwise")
            }
        }
    }

    private var chart: some View {
        Chart(vm.buckets) { bucket in
            BarMark(
                x: .value("Date", bucket.date),
                y: .value("Tokens", bucket.totalTokens)
            )
            .foregroundStyle(by: .value("Model", bucket.model))
        }
        .frame(height: 220)
    }

    private var table: some View {
        Table(vm.buckets) {
            TableColumn("Date", value: \.date)
            TableColumn("Provider", value: \.provider)
            TableColumn("Model", value: \.model)
            TableColumn("Requests") { Text("\($0.requests)") }
            TableColumn("Input") { Text("\($0.input_tokens)") }
            TableColumn("Output") { Text("\($0.output_tokens)") }
            TableColumn("Cache") { Text(String(format: "%.1f%%", $0.cache_ratio * 100)) }
        }
    }
}
