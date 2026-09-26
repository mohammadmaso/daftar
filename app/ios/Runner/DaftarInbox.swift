import Flutter
import UIKit

/// What reaches the app from outside it on iOS (§8.1), answered over the same `daftar/share`
/// channel as Android's MainActivity:
/// * the Share Extension leaves text and images in the App Group's `Inbox/` folder;
/// * the Record widget opens `daftar://record`.
final class DaftarInbox: NSObject, FlutterPlugin, FlutterSceneLifeCycleDelegate {
  static let appGroup = "group.dev.daftar.daftar"

  private let channel: FlutterMethodChannel
  private var recordPending = false

  init(channel: FlutterMethodChannel) {
    self.channel = channel
  }

  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(name: "daftar/share", binaryMessenger: registrar.messenger())
    let inbox = DaftarInbox(channel: channel)
    registrar.addMethodCallDelegate(inbox, channel: channel)
    registrar.addSceneDelegate(inbox)
  }

  static func inboxURL() -> URL? {
    FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroup)?
      .appendingPathComponent("Inbox", isDirectory: true)
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard call.method == "take" else {
      result(FlutterMethodNotImplemented)
      return
    }
    result(take())
  }

  /// Everything waiting, oldest first; taken items are removed.
  private func take() -> [[String: Any]] {
    var out: [[String: Any]] = []
    if recordPending {
      out.append(["record": true])
      recordPending = false
    }
    guard let dir = Self.inboxURL(),
      let files = try? FileManager.default.contentsOfDirectory(
        at: dir, includingPropertiesForKeys: nil)
    else { return out }
    for file in files.sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
      if file.pathExtension == "txt" {
        if let text = try? String(contentsOf: file, encoding: .utf8), !text.isEmpty {
          out.append(["text": text])
        }
      } else if let data = try? Data(contentsOf: file) {
        out.append(["image": FlutterStandardTypedData(bytes: data)])
      }
      try? FileManager.default.removeItem(at: file)
    }
    return out
  }

  private func open(_ contexts: Set<UIOpenURLContext>) -> Bool {
    var handled = false
    for context in contexts where context.url.scheme == "daftar" {
      switch context.url.host {
      case "record":
        recordPending = true
        handled = true
      case "share":
        handled = true
      default:
        break  // e.g. OAuth callbacks belong to other plugins
      }
    }
    if handled { channel.invokeMethod("arrived", arguments: nil) }
    return handled
  }

  func scene(
    _ scene: UIScene, willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions?
  ) -> Bool {
    // Cold start from the widget: Dart's first `take` picks the request up.
    if let contexts = connectionOptions?.urlContexts, !contexts.isEmpty { _ = open(contexts) }
    return false
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) -> Bool {
    open(URLContexts)
  }

  func sceneDidBecomeActive(_ scene: UIScene) {
    // Something may have been shared while the app was in the background.
    channel.invokeMethod("arrived", arguments: nil)
  }
}
