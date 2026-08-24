# Help: I can't open any files

## Linux: Clicking links isn't working

These features are provided by XDG desktop portals, specifically:

- `org.freedesktop.portal.FileChooser`
- `org.freedesktop.portal.OpenURI`

Some window managers, such as `Hyprland`, don't provide a file picker by
default. See
[this list](https://wiki.archlinux.org/title/XDG_Desktop_Portal#List_of_backends_and_interfaces)
as a starting point for alternatives.

## Linux: Could not start inotify

MonyaCode relies on inotify to watch your filesystem for changes. If you cannot start
inotify then MonyaCode will not work reliably.

If you are seeing "too many open files" then first try `sysctl fs.inotify`.

- You should see that max_user_instances is 128 or higher (you can change the
  limit with `sudo sysctl fs.inotify.max_user_instances=1024`). MonyaCode needs only
  1 inotify instance.
- You should see that `max_user_watches` is 8000 or higher (you can change the
  limit with `sudo sysctl fs.inotify.max_user_watches=64000`). MonyaCode needs one
  watch per directory in all your open projects + one per git repository + a
  handful more for settings, themes, keymaps, extensions.

It is also possible that you are running out of file descriptors. You can check
the limits with `ulimit` and update them by editing `/etc/security/limits.conf`.

## Mac: Clicking documentation links isn't working

If you build and run from source and haven't already installed MonyaCode as an
application, the `monyacode://` links will not work properly. To get them working,
you will need to either build an application bundle yourself or install one of
the pre-built bundles from the [MonyaCode website](https://monyacode-editor.com).
