import Foundation

// The generated contract types are logic-free. Identity for SwiftUI lists/charts
// is layered on here (provider + model + date is unique per stats bucket).
extension StatBucketDto: Identifiable {
    public var id: String { "\(provider)|\(model)|\(date)" }

    /// Total tokens (input + output) as a plottable Int.
    var totalTokens: Int { Int(input_tokens + output_tokens) }
}
