import Carbon
import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private var hotKey: GlobalHotKey?

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)

    // ⌥⇧⌘N anywhere: bring the app forward and start a voice note (§8.2).
    let channel = FlutterMethodChannel(
      name: "daftar/hotkey", binaryMessenger: flutterViewController.engine.binaryMessenger)
    hotKey = GlobalHotKey { [weak self] in
      NSApp.activate(ignoringOtherApps: true)
      self?.makeKeyAndOrderFront(nil)
      channel.invokeMethod("record", arguments: nil)
    }

    super.awakeFromNib()
  }
}

/// A system-wide shortcut through Carbon's hot-key API, which needs no accessibility permission.
final class GlobalHotKey {
  private static var onPress: (() -> Void)?
  private var ref: EventHotKeyRef?
  private var handler: EventHandlerRef?

  init(onPress: @escaping () -> Void) {
    GlobalHotKey.onPress = onPress
    var spec = EventTypeSpec(
      eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
    InstallEventHandler(
      GetApplicationEventTarget(),
      { _, _, _ in
        GlobalHotKey.onPress?()
        return noErr
      }, 1, &spec, nil, &handler)
    let id = EventHotKeyID(signature: OSType(0x4441_4654), id: 1)  // "DAFT"
    RegisterEventHotKey(
      UInt32(kVK_ANSI_N), UInt32(cmdKey | optionKey | shiftKey), id,
      GetApplicationEventTarget(), 0, &ref)
  }

  deinit {
    if let ref { UnregisterEventHotKey(ref) }
    if let handler { RemoveEventHandler(handler) }
  }
}
