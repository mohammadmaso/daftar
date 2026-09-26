import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  // The app stays in the menu bar with its window closed, so ⌥⇧⌘N keeps working (ADR-0028).
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return false
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }

  /// Clicking the Dock icon brings the hidden window back.
  override func applicationShouldHandleReopen(
    _ sender: NSApplication, hasVisibleWindows flag: Bool
  ) -> Bool {
    if !flag { (mainFlutterWindow as? MainFlutterWindow)?.present() }
    return true
  }

  /// Files opened with the app (Finder's Open With, or dropped on the Dock icon) are previewed
  /// for import. FlutterAppDelegate implements `openURLs`, so files arrive here too (macOS then
  /// never calls `openFiles`); other URLs, such as the OAuth callback, go to the plugins.
  override func application(_ application: NSApplication, open urls: [URL]) {
    let files = urls.filter(\.isFileURL)
    for url in files {
      (mainFlutterWindow as? MainFlutterWindow)?.importFile(url.path)
    }
    let others = urls.filter { !$0.isFileURL }
    if !others.isEmpty { super.application(application, open: others) }
  }
}
