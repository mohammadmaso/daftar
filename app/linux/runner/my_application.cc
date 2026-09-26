#include "my_application.h"

#include <flutter_linux/flutter_linux.h>
#ifdef GDK_WINDOWING_X11
#include <gdk/gdkx.h>
#endif

#include "desktop_shell.h"
#include "flutter/generated_plugin_registrant.h"

struct _MyApplication {
  GtkApplication parent_instance;
  char** dart_entrypoint_arguments;
  GtkWindow* window;
  DesktopShell* shell;
};

G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

// Called when first Flutter frame received.
static void first_frame_cb(MyApplication* self, FlView* view) {
  gtk_widget_show(gtk_widget_get_toplevel(GTK_WIDGET(view)));
}

// With a tray icon, closing the window hides it so the record shortcut keeps
// working; Quit is in the tray menu.
static gboolean on_delete(GtkWidget* widget, GdkEvent*, gpointer data) {
  MyApplication* self = MY_APPLICATION(data);
  if (self->shell != nullptr && desktop_shell_has_tray(self->shell)) {
    gtk_widget_hide(widget);
    return TRUE;
  }
  return FALSE;
}

static void create_window(MyApplication* self) {
  GApplication* application = G_APPLICATION(self);
  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));

  // Use a header bar when running in GNOME as this is the common style used
  // by applications and is the setup most users will be using (e.g. Ubuntu
  // desktop).
  // If running on X and not using GNOME then just use a traditional title bar
  // in case the window manager does more exotic layout, e.g. tiling.
  // If running on Wayland assume the header bar will work (may need changing
  // if future cases occur).
  gboolean use_header_bar = TRUE;
#ifdef GDK_WINDOWING_X11
  GdkScreen* screen = gtk_window_get_screen(window);
  if (GDK_IS_X11_SCREEN(screen)) {
    const gchar* wm_name = gdk_x11_screen_get_window_manager_name(screen);
    if (g_strcmp0(wm_name, "GNOME Shell") != 0) {
      use_header_bar = FALSE;
    }
  }
#endif
  if (use_header_bar) {
    GtkHeaderBar* header_bar = GTK_HEADER_BAR(gtk_header_bar_new());
    gtk_widget_show(GTK_WIDGET(header_bar));
    gtk_header_bar_set_title(header_bar, "Daftar");
    gtk_header_bar_set_show_close_button(header_bar, TRUE);
    gtk_window_set_titlebar(window, GTK_WIDGET(header_bar));
  } else {
    gtk_window_set_title(window, "Daftar");
  }

  gtk_window_set_default_size(window, 1280, 720);

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(
      project, self->dart_entrypoint_arguments);

  FlView* view = fl_view_new(project);
  GdkRGBA background_color;
  // Background defaults to black, override it here if necessary, e.g. #00000000
  // for transparent.
  gdk_rgba_parse(&background_color, "#000000");
  fl_view_set_background_color(view, &background_color);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  // Show the window when Flutter renders.
  // Requires the view to be realized so we can start rendering.
  g_signal_connect_swapped(view, "first-frame", G_CALLBACK(first_frame_cb),
                           self);
  gtk_widget_realize(GTK_WIDGET(view));

  fl_register_plugins(FL_PLUGIN_REGISTRY(view));

  gtk_widget_grab_focus(GTK_WIDGET(view));

  self->window = window;
  self->shell = desktop_shell_new(GTK_APPLICATION(application), window, view);
  g_signal_connect(window, "delete-event", G_CALLBACK(on_delete), self);
}

// Implements GApplication::activate: a second launch brings the window back.
static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  if (self->window == nullptr) {
    create_window(self);
  } else {
    desktop_shell_present(self->shell);
  }
}

// Implements GApplication::command_line. The app is single-instance, so this
// runs in the first instance for every launch:
//   daftar --record        start a voice note in the compact recorder
//   daftar --import FILE   preview FILE for import
//   daftar FILE…           the same, from "Open with" in a file manager
// Other arguments go to Dart on the first launch.
static int my_application_command_line(GApplication* application,
                                       GApplicationCommandLine* cmdline) {
  MyApplication* self = MY_APPLICATION(application);
  gint argc = 0;
  g_auto(GStrv) argv = g_application_command_line_get_arguments(cmdline, &argc);
  gboolean record = FALSE;
  g_autoptr(GPtrArray) files = g_ptr_array_new_with_free_func(g_free);
  g_autoptr(GPtrArray) rest = g_ptr_array_new_with_free_func(g_free);
  for (gint i = 1; i < argc; ++i) {
    if (g_strcmp0(argv[i], "--record") == 0) {
      record = TRUE;
      continue;
    }
    const gchar* candidate = argv[i];
    gboolean forced = FALSE;
    if (g_strcmp0(argv[i], "--import") == 0 && i + 1 < argc) {
      candidate = argv[++i];
      forced = TRUE;
    }
    g_autoptr(GFile) file =
        g_application_command_line_create_file_for_arg(cmdline, candidate);
    gchar* path = g_file_get_path(file);
    if (path != nullptr && (forced || (candidate[0] != '-' &&
                                       g_file_test(path, G_FILE_TEST_IS_REGULAR)))) {
      g_ptr_array_add(files, path);
    } else {
      g_free(path);
      g_ptr_array_add(rest, g_strdup(argv[i]));
    }
  }
  if (self->window == nullptr) {
    g_ptr_array_add(rest, nullptr);
    self->dart_entrypoint_arguments =
        g_strdupv(reinterpret_cast<gchar**>(rest->pdata));
    create_window(self);
  } else if (!record && files->len == 0) {
    desktop_shell_present(self->shell);
  }
  if (record) desktop_shell_record(self->shell);
  for (guint i = 0; i < files->len; ++i) {
    desktop_shell_import(self->shell,
                         static_cast<const gchar*>(g_ptr_array_index(files, i)));
  }
  return 0;
}

// Implements GApplication::startup.
static void my_application_startup(GApplication* application) {
  // MyApplication* self = MY_APPLICATION(object);

  // Perform any actions required at application startup.

  G_APPLICATION_CLASS(my_application_parent_class)->startup(application);
}

// Implements GApplication::shutdown.
static void my_application_shutdown(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  desktop_shell_free(self->shell);
  self->shell = nullptr;

  G_APPLICATION_CLASS(my_application_parent_class)->shutdown(application);
}

// Implements GObject::dispose.
static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->command_line = my_application_command_line;
  G_APPLICATION_CLASS(klass)->startup = my_application_startup;
  G_APPLICATION_CLASS(klass)->shutdown = my_application_shutdown;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}

static void my_application_init(MyApplication* self) {}

MyApplication* my_application_new() {
  // Set the program name to the application ID, which helps various systems
  // like GTK and desktop environments map this running application to its
  // corresponding .desktop file. This ensures better integration by allowing
  // the application to be recognized beyond its binary name.
  g_set_prgname(APPLICATION_ID);

  return MY_APPLICATION(g_object_new(my_application_get_type(),
                                     "application-id", APPLICATION_ID, "flags",
                                     G_APPLICATION_HANDLES_COMMAND_LINE,
                                     nullptr));
}
