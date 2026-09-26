import Carbon
import Cocoa
import FlutterMacOS

/// The main window, plus desktop integration over the `daftar/hotkey` channel (ADR-0024,
/// ADR-0028): the ⌥⇧⌘N shortcut, the menu-bar item, and the compact recorder the window turns
/// into while a shortcut recording runs.
///
/// Native → Dart: "record", "import" (a path, or nil to pick one).
/// Dart → native: "ready", "miniRecorder" (returns true), "leaveMini" ({open}), "recording"
/// (bool), "labels" (menu titles).
class MainFlutterWindow: NSWindow, NSWindowDelegate {
  private var hotKey: GlobalHotKey?
  private var channel: FlutterMethodChannel?
  private var statusItem: NSStatusItem?
  private var recordItem: NSMenuItem?
  private var importItem: NSMenuItem?
  private var openItem: NSMenuItem?
  private var quitItem: NSMenuItem?

  // Messages from before Dart listened (a cold start from a file or the menu).
  private var dartReady = false
  private var pending: [(String, Any?)] = []

  // The compact recorder.
  private static let miniSize = NSSize(width: 420, height: 150)
  private var mini = false
  private var savedFrame = NSRect.zero
  private var savedStyle: NSWindow.StyleMask = []
  private var savedLevel: NSWindow.Level = .normal
  private var savedBehavior: NSWindow.CollectionBehavior = []
  private var wasVisible = true

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)

    let channel = FlutterMethodChannel(
      name: "daftar/hotkey", binaryMessenger: flutterViewController.engine.binaryMessenger)
    self.channel = channel
    channel.setMethodCallHandler { [weak self] call, result in
      self?.handle(call, result: result)
    }

    // ⌥⇧⌘N anywhere: the compact recorder starts a voice note (§8.2).
    hotKey = GlobalHotKey { [weak self] in
      self?.record()
    }

    delegate = self
    setUpStatusItem()
    super.awakeFromNib()
  }

  // MARK: Messages

  private func send(_ method: String, _ argument: Any? = nil) {
    guard dartReady, let channel else {
      pending.append((method, argument))
      return
    }
    channel.invokeMethod(method, arguments: argument)
  }

  func record() {
    present()
    send("record")
  }

  /// A file opened with the app (Finder's Open With, or dropped on the Dock icon), or nil to pick.
  func importFile(_ path: String?) {
    present()
    send("import", path)
  }

  func present() {
    NSApp.activate(ignoringOtherApps: true)
    makeKeyAndOrderFront(nil)
  }

  private func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "ready":
      dartReady = true
      let queued = pending
      pending.removeAll()
      for (method, argument) in queued { send(method, argument) }
      result(nil)
    case "miniRecorder":
      enterMini()
      result(true)
    case "leaveMini":
      let open = (call.arguments as? [String: Any])?["open"] as? Bool ?? false
      leaveMini(open: open)
      result(nil)
    case "recording":
      setRecording(call.arguments as? Bool ?? false)
      result(nil)
    case "labels":
      if let labels = call.arguments as? [String: String] { setLabels(labels) }
      result(nil)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  // MARK: Closing hides to the menu bar, so the shortcut keeps working

  func windowShouldClose(_ sender: NSWindow) -> Bool {
    orderOut(nil)
    return false
  }

  // MARK: Compact recorder

  private func enterMini() {
    if !mini {
      savedFrame = frame
      savedStyle = styleMask
      savedLevel = level
      savedBehavior = collectionBehavior
      wasVisible = isVisible
    }
    mini = true
    styleMask.insert(.fullSizeContentView)
    titleVisibility = .hidden
    titlebarAppearsTransparent = true
    for button in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
      standardWindowButton(button)?.isHidden = true
    }
    isMovableByWindowBackground = true
    level = .floating
    collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
    let visible = (NSScreen.main ?? screen)?.visibleFrame ?? frame
    let size = MainFlutterWindow.miniSize
    setFrame(
      NSRect(
        x: visible.midX - size.width / 2, y: visible.maxY - size.height - 48,
        width: size.width, height: size.height),
      display: true, animate: false)
    present()
  }

  private func leaveMini(open: Bool) {
    guard mini else {
      if open { present() }
      return
    }
    mini = false
    styleMask = savedStyle
    titleVisibility = .visible
    titlebarAppearsTransparent = false
    for button in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
      standardWindowButton(button)?.isHidden = false
    }
    isMovableByWindowBackground = false
    level = savedLevel
    collectionBehavior = savedBehavior
    setFrame(savedFrame, display: true, animate: false)
    if open || wasVisible {
      present()
    } else {
      orderOut(nil)
    }
  }

  // MARK: Menu-bar item

  private func setUpStatusItem() {
    let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
    item.button?.image = MenuBarIcon.idle
    item.button?.toolTip = Bundle.main.object(forInfoDictionaryKey: "CFBundleName") as? String
    let menu = NSMenu()
    let record = NSMenuItem(
      title: "Record a voice note", action: #selector(menuRecord), keyEquivalent: "n")
    record.keyEquivalentModifierMask = [.command, .option, .shift]
    let importFile = NSMenuItem(
      title: "Import a file…", action: #selector(menuImport), keyEquivalent: "")
    let open = NSMenuItem(title: "Open", action: #selector(menuOpen), keyEquivalent: "")
    let quit = NSMenuItem(title: "Quit", action: #selector(menuQuit), keyEquivalent: "q")
    for entry in [record, importFile, open] {
      entry.target = self
      menu.addItem(entry)
    }
    menu.addItem(NSMenuItem.separator())
    quit.target = self
    menu.addItem(quit)
    item.menu = menu
    statusItem = item
    recordItem = record
    importItem = importFile
    openItem = open
    quitItem = quit
  }

  private func setLabels(_ labels: [String: String]) {
    if let t = labels["record"] { recordItem?.title = t }
    if let t = labels["import"] { importItem?.title = t }
    if let t = labels["open"] { openItem?.title = t }
    if let t = labels["quit"] { quitItem?.title = t }
  }

  private func setRecording(_ on: Bool) {
    statusItem?.button?.image = on ? MenuBarIcon.recording : MenuBarIcon.idle
  }

  @objc private func menuRecord() { record() }
  @objc private func menuImport() { importFile(nil) }
  @objc private func menuOpen() { present() }
  @objc private func menuQuit() { NSApp.terminate(nil) }
}

/// Menu-bar glyphs drawn in code as template images (they follow the menu bar's appearance):
/// the app's notebook page, and a filled dot while recording.
enum MenuBarIcon {
  static let idle = make { rect in
    let page = NSBezierPath(
      roundedRect: NSRect(x: 4, y: 2, width: 10, height: 14), xRadius: 1.5, yRadius: 1.5)
    page.lineWidth = 1.4
    page.stroke()
    for y in [6.0, 9.0, 12.0] {
      let line = NSBezierPath()
      line.move(to: NSPoint(x: 6.5, y: y))
      line.line(to: NSPoint(x: 11.5, y: y))
      line.lineWidth = 1.2
      line.stroke()
    }
  }

  static let recording = make { rect in
    NSBezierPath(ovalIn: NSRect(x: 4, y: 4, width: 10, height: 10)).fill()
  }

  private static func make(_ draw: @escaping (NSRect) -> Void) -> NSImage {
    let image = NSImage(size: NSSize(width: 18, height: 18), flipped: false) { rect in
      NSColor.black.set()
      draw(rect)
      return true
    }
    image.isTemplate = true
    return image
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
