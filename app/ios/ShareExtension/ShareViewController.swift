import UIKit
import UniformTypeIdentifiers

/// Share into Daftar (§8.1): text, links and images are left in the App Group's `Inbox/` folder,
/// and the app files them as raw captures the next time it is active. Nothing is sent anywhere.
final class ShareViewController: UIViewController {
  private static let appGroup = "group.dev.daftar.daftar"

  override func viewDidAppear(_ animated: Bool) {
    super.viewDidAppear(animated)
    collect { [weak self] in
      self?.extensionContext?.completeRequest(returningItems: nil)
    }
  }

  private func collect(done: @escaping () -> Void) {
    guard
      let inbox = FileManager.default.containerURL(
        forSecurityApplicationGroupIdentifier: Self.appGroup)?
        .appendingPathComponent("Inbox", isDirectory: true)
    else { return done() }
    try? FileManager.default.createDirectory(at: inbox, withIntermediateDirectories: true)

    let items = extensionContext?.inputItems as? [NSExtensionItem] ?? []
    let group = DispatchGroup()
    let lock = NSLock()
    var texts: [String] = []
    func addText(_ text: String) {
      let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
      guard !t.isEmpty else { return }
      lock.lock()
      if !texts.contains(t) { texts.append(t) }
      lock.unlock()
    }

    for item in items {
      if let text = item.attributedContentText?.string { addText(text) }
      for provider in item.attachments ?? [] {
        if provider.hasItemConformingToTypeIdentifier(UTType.image.identifier) {
          group.enter()
          provider.loadDataRepresentation(forTypeIdentifier: UTType.image.identifier) { data, _ in
            if let data { Self.write(data, ext: "img", to: inbox) }
            group.leave()
          }
        } else if provider.hasItemConformingToTypeIdentifier(UTType.url.identifier) {
          group.enter()
          provider.loadItem(forTypeIdentifier: UTType.url.identifier) { value, _ in
            if let url = value as? URL { addText(url.absoluteString) }
            group.leave()
          }
        } else if provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier) {
          group.enter()
          provider.loadItem(forTypeIdentifier: UTType.plainText.identifier) { value, _ in
            if let text = value as? String { addText(text) }
            group.leave()
          }
        }
      }
    }
    group.notify(queue: .main) {
      if !texts.isEmpty, let data = texts.joined(separator: "\n\n").data(using: .utf8) {
        Self.write(data, ext: "txt", to: inbox)
      }
      done()
    }
  }

  private static func write(_ data: Data, ext: String, to inbox: URL) {
    let name = "\(Int(Date().timeIntervalSince1970 * 1000))-\(UUID().uuidString).\(ext)"
    try? data.write(to: inbox.appendingPathComponent(name), options: .atomic)
  }
}
