import SwiftUI
import WidgetKit

/// The "Record" home-screen widget (§8.1): one tap opens the app straight into recording, through
/// `daftar://record` (Runner/DaftarInbox.swift).
@main
struct RecordWidget: Widget {
  var body: some WidgetConfiguration {
    StaticConfiguration(kind: "RecordWidget", provider: Provider()) { _ in
      RecordView()
    }
    .configurationDisplayName(Strings.record)
    .description(Strings.description)
    .supportedFamilies([.systemSmall])
  }
}

struct Entry: TimelineEntry {
  let date: Date
}

struct Provider: TimelineProvider {
  func placeholder(in context: Context) -> Entry { Entry(date: Date()) }

  func getSnapshot(in context: Context, completion: @escaping (Entry) -> Void) {
    completion(Entry(date: Date()))
  }

  func getTimeline(in context: Context, completion: @escaping (Timeline<Entry>) -> Void) {
    completion(Timeline(entries: [Entry(date: Date())], policy: .never))
  }
}

/// Mirrors the accent and paper tokens (lib/design/tokens.dart), light and dark.
private enum Palette {
  static let accent = Color(
    UIColor { $0.userInterfaceStyle == .dark
      ? UIColor(red: 0x86 / 255, green: 0xB6 / 255, blue: 0xC2 / 255, alpha: 1)
      : UIColor(red: 0x22 / 255, green: 0x50 / 255, blue: 0x5E / 255, alpha: 1) })
  static let onAccent = Color(
    UIColor { $0.userInterfaceStyle == .dark
      ? UIColor(red: 0x12 / 255, green: 0x29 / 255, blue: 0x2F / 255, alpha: 1)
      : UIColor(red: 0xF7 / 255, green: 0xF4 / 255, blue: 0xEE / 255, alpha: 1) })
}

/// Widget text follows the device language (English or Persian).
private enum Strings {
  static var persian: Bool { Locale.preferredLanguages.first?.hasPrefix("fa") ?? false }
  static var record: String { persian ? "ضبط" : "Record" }
  static var description: String {
    persian ? "با یک ضربه یادداشت صوتی را شروع کنید" : "Start a voice note in one tap"
  }
}

/// The app's own stroke microphone (lib/design/icons.dart), not a system symbol.
private struct Mic: Shape {
  func path(in r: CGRect) -> Path {
    let u = r.width / 24
    var p = Path()
    p.addRoundedRect(
      in: CGRect(x: 9 * u, y: 3 * u, width: 6 * u, height: 11 * u),
      cornerSize: CGSize(width: 3 * u, height: 3 * u))
    p.move(to: CGPoint(x: 6 * u, y: 11 * u))
    p.addArc(
      center: CGPoint(x: 12 * u, y: 11 * u), radius: 6 * u, startAngle: .degrees(180),
      endAngle: .degrees(0), clockwise: true)
    p.move(to: CGPoint(x: 12 * u, y: 17 * u))
    p.addLine(to: CGPoint(x: 12 * u, y: 21 * u))
    p.move(to: CGPoint(x: 9 * u, y: 21 * u))
    p.addLine(to: CGPoint(x: 15 * u, y: 21 * u))
    return p
  }
}

struct RecordView: View {
  var body: some View {
    let content = VStack(spacing: 10) {
      Mic()
        .stroke(Palette.onAccent, style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
        .frame(width: 40, height: 40)
      Text(Strings.record)
        .font(.headline)
        .foregroundColor(Palette.onAccent)
    }
    .widgetURL(URL(string: "daftar://record"))
    .accessibilityElement(children: .combine)
    .accessibilityLabel(Strings.record)

    if #available(iOSApplicationExtension 17.0, *) {
      content.containerBackground(Palette.accent, for: .widget)
    } else {
      ZStack {
        Palette.accent
        content
      }
    }
  }
}
