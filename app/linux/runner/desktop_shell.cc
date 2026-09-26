#include "desktop_shell.h"

#include <dlfcn.h>
#include <string.h>
#include <unistd.h>

#include <string>
#include <vector>

#ifdef GDK_WINDOWING_X11
#include <X11/Xlib.h>
#include <X11/keysym.h>
#include <gdk/gdkx.h>
#endif

namespace {

constexpr char kChannel[] = "daftar/hotkey";
constexpr int kMiniWidth = 420;
constexpr int kMiniHeight = 150;

// ─────────────── libayatana-appindicator, loaded at run time ───────────────

typedef struct _AppIndicator AppIndicator;
using IndicatorNew = AppIndicator* (*)(const gchar*, const gchar*, int);
using IndicatorSetStatus = void (*)(AppIndicator*, int);
using IndicatorSetMenu = void (*)(AppIndicator*, GtkMenu*);
using IndicatorSetIcon = void (*)(AppIndicator*, const gchar*, const gchar*);
using IndicatorSetTitle = void (*)(AppIndicator*, const gchar*);

constexpr int kCategoryApplicationStatus = 0;
constexpr int kStatusActive = 1;

struct Indicator {
  void* lib = nullptr;
  IndicatorNew create = nullptr;
  IndicatorSetStatus set_status = nullptr;
  IndicatorSetMenu set_menu = nullptr;
  IndicatorSetIcon set_icon = nullptr;
  IndicatorSetTitle set_title = nullptr;

  bool Load() {
    for (const char* name : {"libayatana-appindicator3.so.1",
                             "libappindicator3.so.1"}) {
      lib = dlopen(name, RTLD_LAZY | RTLD_LOCAL);
      if (lib != nullptr) break;
    }
    if (lib == nullptr) return false;
    create = reinterpret_cast<IndicatorNew>(dlsym(lib, "app_indicator_new"));
    set_status = reinterpret_cast<IndicatorSetStatus>(
        dlsym(lib, "app_indicator_set_status"));
    set_menu = reinterpret_cast<IndicatorSetMenu>(
        dlsym(lib, "app_indicator_set_menu"));
    set_icon = reinterpret_cast<IndicatorSetIcon>(
        dlsym(lib, "app_indicator_set_icon_full"));
    set_title = reinterpret_cast<IndicatorSetTitle>(
        dlsym(lib, "app_indicator_set_title"));
    return create && set_status && set_menu && set_icon;
  }
};

// The bundle's data directory (next to the executable), where the tray icons
// are installed.
std::string DataDir() {
  char buf[4096];
  ssize_t n = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
  if (n <= 0) return "";
  buf[n] = '\0';
  g_autofree gchar* dir = g_path_get_dirname(buf);
  return std::string(dir) + "/data";
}

// A message for Dart that arrived before Dart listened.
struct Pending {
  std::string method;
  std::string arg;
  bool has_arg;
};

}  // namespace

struct _DesktopShell {
  GtkApplication* app;
  GtkWindow* window;
  FlMethodChannel* channel;

  // Actions from before Dart listened (a cold start through `--record`).
  bool dart_ready;
  std::vector<Pending> pending;

  Indicator indicator;
  AppIndicator* tray;
  GtkWidget* menu;
  GtkWidget* item_record;
  GtkWidget* item_import;
  GtkWidget* item_open;
  GtkWidget* item_quit;
  std::string icon_idle;
  std::string icon_recording;

  // Compact recorder.
  bool mini;
  bool mini_was_visible;
  gint saved_x, saved_y, saved_w, saved_h;
  gboolean saved_decorated;

  // Global shortcut: "x11", "portal", "taken" or "none".
  const char* hotkey_kind;
  unsigned long x11_keycode;
  GDBusConnection* bus;
  gchar* portal_session;
  guint portal_activated_sub;
  guint portal_response_sub;
};

// ─────────────── Dart channel ───────────────

static void send(DesktopShell* self, const char* method, const gchar* arg,
                 bool has_arg) {
  g_debug("desktop shell: %s %s (dart %s)", method, arg ? arg : "",
          self->dart_ready ? "ready" : "not ready yet");
  if (!self->dart_ready) {
    self->pending.push_back({method, arg ? arg : "", has_arg && arg});
    return;
  }
  g_autoptr(FlValue) value =
      has_arg && arg ? fl_value_new_string(arg) : fl_value_new_null();
  fl_method_channel_invoke_method(self->channel, method, value, nullptr,
                                  nullptr, nullptr);
}

void desktop_shell_record(DesktopShell* self) {
  desktop_shell_present(self);
  send(self, "record", nullptr, false);
}

void desktop_shell_import(DesktopShell* self, const gchar* path) {
  desktop_shell_present(self);
  send(self, "import", path, path != nullptr);
}

void desktop_shell_present(DesktopShell* self) {
  gtk_widget_show(GTK_WIDGET(self->window));
  gtk_window_present(self->window);
}

gboolean desktop_shell_has_tray(DesktopShell* self) {
  return self->tray != nullptr;
}

static void set_recording(DesktopShell* self, bool on) {
  if (self->tray == nullptr) return;
  self->indicator.set_icon(
      self->tray, (on ? self->icon_recording : self->icon_idle).c_str(), "");
}

// ─────────────── Compact recorder ───────────────

// GTK recomputes the client-side decoration margins only after a decoration
// change is processed; sizing in the same turn is off by the shadow width.
struct Resize {
  DesktopShell* shell;
  gint w, h, x, y;
  bool move;
};

static gboolean apply_resize(gpointer data) {
  auto* r = static_cast<Resize*>(data);
  gtk_window_resize(r->shell->window, r->w, r->h);
  if (r->move) gtk_window_move(r->shell->window, r->x, r->y);
  delete r;
  return G_SOURCE_REMOVE;
}

static void resize_later(DesktopShell* self, gint w, gint h, bool move, gint x,
                         gint y) {
  g_idle_add(apply_resize, new Resize{self, w, h, x, y, move});
}

static void enter_mini(DesktopShell* self) {
  GtkWindow* w = self->window;
  if (!self->mini) {
    self->mini_was_visible = gtk_widget_get_visible(GTK_WIDGET(w));
    gtk_window_get_position(w, &self->saved_x, &self->saved_y);
    gtk_window_get_size(w, &self->saved_w, &self->saved_h);
    self->saved_decorated = gtk_window_get_decorated(w);
  }
  self->mini = true;
  gtk_window_set_decorated(w, FALSE);
  gtk_window_set_keep_above(w, TRUE);
  gtk_window_set_skip_taskbar_hint(w, TRUE);
  // Top centre of the monitor under the pointer (ignored on Wayland, where
  // the compositor places windows).
  GdkDisplay* display = gdk_display_get_default();
  GdkSeat* seat = gdk_display_get_default_seat(display);
  gint px = 0, py = 0;
  if (seat != nullptr) {
    gdk_device_get_position(gdk_seat_get_pointer(seat), nullptr, &px, &py);
  }
  GdkMonitor* monitor = gdk_display_get_monitor_at_point(display, px, py);
  GdkRectangle area = {0, 0, 0, 0};
  if (monitor != nullptr) gdk_monitor_get_workarea(monitor, &area);
  resize_later(self, kMiniWidth, kMiniHeight, monitor != nullptr,
               area.x + (area.width - kMiniWidth) / 2, area.y + 48);
  desktop_shell_present(self);
}

static void leave_mini(DesktopShell* self, bool open) {
  if (!self->mini) {
    if (open) desktop_shell_present(self);
    return;
  }
  self->mini = false;
  GtkWindow* w = self->window;
  gtk_window_set_keep_above(w, FALSE);
  gtk_window_set_skip_taskbar_hint(w, FALSE);
  gtk_window_set_decorated(w, self->saved_decorated);
  resize_later(self, self->saved_w, self->saved_h, true, self->saved_x,
               self->saved_y);
  if (open || self->mini_was_visible) {
    desktop_shell_present(self);
  } else {
    gtk_widget_hide(GTK_WIDGET(w));
  }
}

// ─────────────── Tray ───────────────

static void on_menu_record(GtkMenuItem*, gpointer data) {
  desktop_shell_record(static_cast<DesktopShell*>(data));
}

static void on_menu_import(GtkMenuItem*, gpointer data) {
  desktop_shell_import(static_cast<DesktopShell*>(data), nullptr);
}

static void on_menu_open(GtkMenuItem*, gpointer data) {
  desktop_shell_present(static_cast<DesktopShell*>(data));
}

static void on_menu_quit(GtkMenuItem*, gpointer data) {
  g_application_quit(G_APPLICATION(static_cast<DesktopShell*>(data)->app));
}

static GtkWidget* menu_item(GtkWidget* menu, const char* label,
                            GCallback callback, DesktopShell* self) {
  GtkWidget* item = gtk_menu_item_new_with_label(label);
  g_signal_connect(item, "activate", callback, self);
  gtk_menu_shell_append(GTK_MENU_SHELL(menu), item);
  gtk_widget_show(item);
  return item;
}

static void setup_tray(DesktopShell* self) {
  if (!self->indicator.Load()) {
    g_message("No appindicator library; running without a tray icon.");
    return;
  }
  std::string data = DataDir();
  self->icon_idle = data + "/tray.png";
  self->icon_recording = data + "/tray-recording.png";
  if (!g_file_test(self->icon_idle.c_str(), G_FILE_TEST_EXISTS)) {
    self->icon_idle = "dev.daftar.Daftar";
    self->icon_recording = "media-record";
  }
  self->tray = self->indicator.create(APPLICATION_ID, self->icon_idle.c_str(),
                                      kCategoryApplicationStatus);
  if (self->tray == nullptr) return;
  self->menu = gtk_menu_new();
  self->item_record = menu_item(self->menu, "Record a voice note",
                                G_CALLBACK(on_menu_record), self);
  self->item_import = menu_item(self->menu, "Import a file…",
                                G_CALLBACK(on_menu_import), self);
  self->item_open =
      menu_item(self->menu, "Open", G_CALLBACK(on_menu_open), self);
  GtkWidget* sep = gtk_separator_menu_item_new();
  gtk_menu_shell_append(GTK_MENU_SHELL(self->menu), sep);
  gtk_widget_show(sep);
  self->item_quit =
      menu_item(self->menu, "Quit", G_CALLBACK(on_menu_quit), self);
  self->indicator.set_menu(self->tray, GTK_MENU(self->menu));
  if (self->indicator.set_title) {
    self->indicator.set_title(self->tray, g_get_application_name());
  }
  self->indicator.set_status(self->tray, kStatusActive);
}

static void set_labels(DesktopShell* self, FlValue* map) {
  if (self->tray == nullptr || fl_value_get_type(map) != FL_VALUE_TYPE_MAP) {
    return;
  }
  auto apply = [&](const char* key, GtkWidget* item) {
    FlValue* v = fl_value_lookup_string(map, key);
    if (v != nullptr && fl_value_get_type(v) == FL_VALUE_TYPE_STRING) {
      gtk_menu_item_set_label(GTK_MENU_ITEM(item), fl_value_get_string(v));
    }
  };
  apply("record", self->item_record);
  apply("import", self->item_import);
  apply("open", self->item_open);
  apply("quit", self->item_quit);
}

// ─────────────── Global shortcut: X11 ───────────────

#ifdef GDK_WINDOWING_X11
static const unsigned int kX11Mods = ControlMask | Mod1Mask | ShiftMask;
// The shortcut must fire with Caps Lock or Num Lock on as well.
static const unsigned int kLockVariants[] = {0, LockMask, Mod2Mask,
                                             LockMask | Mod2Mask};

static GdkFilterReturn x11_filter(GdkXEvent* xevent, GdkEvent*, gpointer data) {
  auto* self = static_cast<DesktopShell*>(data);
  auto* ev = static_cast<XEvent*>(xevent);
  if (ev->type == KeyPress && ev->xkey.keycode == self->x11_keycode &&
      (ev->xkey.state & kX11Mods) == kX11Mods) {
    desktop_shell_record(self);
    return GDK_FILTER_REMOVE;
  }
  return GDK_FILTER_CONTINUE;
}

static bool setup_x11_hotkey(DesktopShell* self) {
  GdkDisplay* display = gdk_display_get_default();
  if (!GDK_IS_X11_DISPLAY(display)) return false;
  Display* xd = GDK_DISPLAY_XDISPLAY(display);
  Window root = DefaultRootWindow(xd);
  self->x11_keycode = XKeysymToKeycode(xd, XK_n);
  gdk_x11_display_error_trap_push(display);
  for (unsigned int lock : kLockVariants) {
    XGrabKey(xd, self->x11_keycode, kX11Mods | lock, root, False,
             GrabModeAsync, GrabModeAsync);
  }
  if (gdk_x11_display_error_trap_pop(display) != 0) {
    g_message("Ctrl+Alt+Shift+N is taken by another program.");
    self->hotkey_kind = "taken";
    return true;
  }
  gdk_window_add_filter(gdk_get_default_root_window(), x11_filter, self);
  self->hotkey_kind = "x11";
  return true;
}
#endif

// ─────────────── Global shortcut: xdg-desktop-portal ───────────────

constexpr char kPortalBus[] = "org.freedesktop.portal.Desktop";
constexpr char kPortalPath[] = "/org/freedesktop/portal/desktop";
constexpr char kShortcutsIface[] = "org.freedesktop.portal.GlobalShortcuts";

// The Request object path the portal will use for `token`.
static gchar* request_path(DesktopShell* self, const char* token) {
  g_autofree gchar* sender =
      g_strdup(g_dbus_connection_get_unique_name(self->bus) + 1);
  for (gchar* c = sender; *c; ++c) {
    if (*c == '.') *c = '_';
  }
  return g_strdup_printf("%s/request/%s/%s", kPortalPath, sender, token);
}

static void on_activated(GDBusConnection*, const gchar*, const gchar*,
                         const gchar*, const gchar*, GVariant* params,
                         gpointer data) {
  const gchar* id = nullptr;
  g_variant_get_child(params, 1, "&s", &id);
  if (g_strcmp0(id, "record") == 0) {
    desktop_shell_record(static_cast<DesktopShell*>(data));
  }
}

static void bind_shortcuts(DesktopShell* self);

static void on_session_created(GDBusConnection* bus, const gchar*,
                               const gchar*, const gchar*, const gchar*,
                               GVariant* params, gpointer data) {
  auto* self = static_cast<DesktopShell*>(data);
  g_dbus_connection_signal_unsubscribe(bus, self->portal_response_sub);
  self->portal_response_sub = 0;
  guint32 response = 1;
  g_autoptr(GVariant) results = nullptr;
  g_variant_get(params, "(u@a{sv})", &response, &results);
  const gchar* handle = nullptr;
  if (response != 0 ||
      !g_variant_lookup(results, "session_handle", "&s", &handle)) {
    g_message("The desktop declined a global shortcut session.");
    return;
  }
  self->portal_session = g_strdup(handle);
  self->hotkey_kind = "portal";
  self->portal_activated_sub = g_dbus_connection_signal_subscribe(
      bus, kPortalBus, kShortcutsIface, "Activated", kPortalPath, nullptr,
      G_DBUS_SIGNAL_FLAGS_NONE, on_activated, self, nullptr);
  bind_shortcuts(self);
}

static void bind_shortcuts(DesktopShell* self) {
  GVariantBuilder shortcuts;
  g_variant_builder_init(&shortcuts, G_VARIANT_TYPE("a(sa{sv})"));
  GVariantBuilder props;
  g_variant_builder_init(&props, G_VARIANT_TYPE("a{sv}"));
  g_variant_builder_add(&props, "{sv}", "description",
                        g_variant_new_string("Record a voice note"));
  g_variant_builder_add(&props, "{sv}", "preferred_trigger",
                        g_variant_new_string("CTRL+ALT+SHIFT+n"));
  g_variant_builder_add(&shortcuts, "(sa{sv})", "record", &props);
  GVariantBuilder options;
  g_variant_builder_init(&options, G_VARIANT_TYPE("a{sv}"));
  g_variant_builder_add(&options, "{sv}", "handle_token",
                        g_variant_new_string("daftar_bind"));
  g_dbus_connection_call(
      self->bus, kPortalBus, kPortalPath, kShortcutsIface, "BindShortcuts",
      g_variant_new("(oa(sa{sv})sa{sv})", self->portal_session, &shortcuts,
                    "", &options),
      nullptr, G_DBUS_CALL_FLAGS_NONE, -1, nullptr, nullptr, nullptr);
}

static void setup_portal_hotkey(DesktopShell* self) {
  self->bus = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, nullptr);
  if (self->bus == nullptr) return;
  // Outside a sandbox the portal learns our app id from the host registry;
  // older portals don't have it, which is fine.
  g_dbus_connection_call(
      self->bus, kPortalBus, kPortalPath, "org.freedesktop.host.portal.Registry",
      "Register",
      g_variant_new("(sa{sv})", "dev.daftar.Daftar", nullptr),
      nullptr, G_DBUS_CALL_FLAGS_NONE, -1, nullptr, nullptr, nullptr);

  g_autofree gchar* path = request_path(self, "daftar_session");
  self->portal_response_sub = g_dbus_connection_signal_subscribe(
      self->bus, kPortalBus, "org.freedesktop.portal.Request", "Response", path,
      nullptr, G_DBUS_SIGNAL_FLAGS_NO_MATCH_RULE, on_session_created, self,
      nullptr);
  GVariantBuilder options;
  g_variant_builder_init(&options, G_VARIANT_TYPE("a{sv}"));
  g_variant_builder_add(&options, "{sv}", "handle_token",
                        g_variant_new_string("daftar_session"));
  g_variant_builder_add(&options, "{sv}", "session_handle_token",
                        g_variant_new_string("daftar"));
  g_dbus_connection_call(self->bus, kPortalBus, kPortalPath, kShortcutsIface,
                         "CreateSession", g_variant_new("(a{sv})", &options),
                         nullptr, G_DBUS_CALL_FLAGS_NONE, -1, nullptr, nullptr,
                         nullptr);
}

// ─────────────── Global shortcut: a GNOME custom keybinding ───────────────
//
// GNOME before 48 has neither an X11 grab (on Wayland) nor the portal. There,
// on request, the shortcut is added to GNOME's own custom keyboard shortcuts,
// running `daftar --record`, which reaches this instance (single instance).

constexpr char kMediaKeys[] = "org.gnome.settings-daemon.plugins.media-keys";
constexpr char kCustomKey[] =
    "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
constexpr char kOurKeyPath[] =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/"
    "daftar-record/";

static bool gnome_shortcuts_available() {
  // A sandbox can't write the host's settings.
  if (g_getenv("FLATPAK_ID") != nullptr) return false;
  GSettingsSchemaSource* source = g_settings_schema_source_get_default();
  if (source == nullptr) return false;
  g_autoptr(GSettingsSchema) keys =
      g_settings_schema_source_lookup(source, kMediaKeys, TRUE);
  g_autoptr(GSettingsSchema) custom =
      g_settings_schema_source_lookup(source, kCustomKey, TRUE);
  return keys != nullptr && custom != nullptr;
}

static bool gnome_shortcut_installed() {
  if (!gnome_shortcuts_available()) return false;
  g_autoptr(GSettings) keys = g_settings_new(kMediaKeys);
  g_auto(GStrv) paths = g_settings_get_strv(keys, "custom-keybindings");
  return g_strv_contains(paths, kOurKeyPath);
}

// The command a desktop shortcut runs: this AppImage or this executable.
static std::string record_command() {
  const gchar* appimage = g_getenv("APPIMAGE");
  std::string exe;
  if (appimage != nullptr) {
    exe = appimage;
  } else {
    char buf[4096];
    ssize_t n = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
    if (n <= 0) return "";
    buf[n] = '\0';
    exe = buf;
  }
  g_autofree gchar* quoted = g_shell_quote(exe.c_str());
  return std::string(quoted) + " --record";
}

static bool install_gnome_shortcut(const gchar* name) {
  if (!gnome_shortcuts_available()) return false;
  std::string command = record_command();
  if (command.empty()) return false;
  g_autoptr(GSettings) custom =
      g_settings_new_with_path(kCustomKey, kOurKeyPath);
  g_settings_set_string(custom, "name", name);
  g_settings_set_string(custom, "command", command.c_str());
  g_settings_set_string(custom, "binding", "<Control><Alt><Shift>n");
  g_autoptr(GSettings) keys = g_settings_new(kMediaKeys);
  g_auto(GStrv) paths = g_settings_get_strv(keys, "custom-keybindings");
  if (!g_strv_contains(paths, kOurKeyPath)) {
    g_autoptr(GStrvBuilder) builder = g_strv_builder_new();
    g_strv_builder_addv(builder, const_cast<const char**>(paths));
    g_strv_builder_add(builder, kOurKeyPath);
    g_auto(GStrv) updated = g_strv_builder_end(builder);
    g_settings_set_strv(keys, "custom-keybindings", updated);
  }
  g_settings_sync();
  return true;
}

static FlValue* shortcut_status(DesktopShell* self) {
  FlValue* map = fl_value_new_map();
  bool gnome = gnome_shortcuts_available();
  fl_value_set_string_take(map, "kind", fl_value_new_string(self->hotkey_kind));
  fl_value_set_string_take(map, "canInstall", fl_value_new_bool(gnome));
  fl_value_set_string_take(map, "installed",
                           fl_value_new_bool(gnome && gnome_shortcut_installed()));
  return map;
}

// ─────────────── Method calls from Dart ───────────────

static void on_method_call(FlMethodChannel*, FlMethodCall* call,
                           gpointer data) {
  auto* self = static_cast<DesktopShell*>(data);
  const gchar* method = fl_method_call_get_name(call);
  FlValue* args = fl_method_call_get_args(call);
  g_debug("desktop shell: from dart: %s", method);
  g_autoptr(FlValue) result = nullptr;
  if (strcmp(method, "ready") == 0) {
    self->dart_ready = true;
    auto pending = std::move(self->pending);
    self->pending.clear();
    for (const auto& p : pending) {
      send(self, p.method.c_str(), p.arg.c_str(), p.has_arg);
    }
    result = fl_value_new_null();
  } else if (strcmp(method, "miniRecorder") == 0) {
    enter_mini(self);
    result = fl_value_new_bool(TRUE);
  } else if (strcmp(method, "leaveMini") == 0) {
    FlValue* open = args && fl_value_get_type(args) == FL_VALUE_TYPE_MAP
                        ? fl_value_lookup_string(args, "open")
                        : nullptr;
    leave_mini(self, open != nullptr &&
                         fl_value_get_type(open) == FL_VALUE_TYPE_BOOL &&
                         fl_value_get_bool(open));
    result = fl_value_new_null();
  } else if (strcmp(method, "recording") == 0) {
    set_recording(self, args && fl_value_get_type(args) == FL_VALUE_TYPE_BOOL &&
                            fl_value_get_bool(args));
    result = fl_value_new_null();
  } else if (strcmp(method, "shortcut") == 0) {
    result = shortcut_status(self);
  } else if (strcmp(method, "installShortcut") == 0) {
    FlValue* name = args && fl_value_get_type(args) == FL_VALUE_TYPE_STRING
                        ? args
                        : nullptr;
    result = fl_value_new_bool(install_gnome_shortcut(
        name ? fl_value_get_string(name) : "Record a voice note"));
  } else if (strcmp(method, "labels") == 0) {
    if (args) set_labels(self, args);
    result = fl_value_new_null();
  } else {
    fl_method_call_respond_not_implemented(call, nullptr);
    return;
  }
  fl_method_call_respond_success(call, result, nullptr);
}

DesktopShell* desktop_shell_new(GtkApplication* app, GtkWindow* window,
                                FlView* view) {
  auto* self = new DesktopShell();
  self->hotkey_kind = "none";
  self->app = app;
  self->window = window;
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  self->channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)), kChannel,
      FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(self->channel, on_method_call,
                                            self, nullptr);
  setup_tray(self);
  bool x11 = false;
#ifdef GDK_WINDOWING_X11
  x11 = setup_x11_hotkey(self);
#endif
  if (!x11) setup_portal_hotkey(self);
  return self;
}

void desktop_shell_free(DesktopShell* self) {
  if (self == nullptr) return;
  if (self->bus != nullptr) {
    if (self->portal_activated_sub) {
      g_dbus_connection_signal_unsubscribe(self->bus,
                                           self->portal_activated_sub);
    }
    if (self->portal_response_sub) {
      g_dbus_connection_signal_unsubscribe(self->bus,
                                           self->portal_response_sub);
    }
    g_object_unref(self->bus);
  }
  g_free(self->portal_session);
  g_clear_object(&self->channel);
  delete self;
}
