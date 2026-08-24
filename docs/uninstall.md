# Uninstall

This guide covers how to uninstall MonyaCode on different operating systems.

## macOS

### Bundle compilation

If you installed MonyaCode by compiling it using the `script/bundle-mac` command:

1. Quit MonyaCode if it's running
2. Open Finder and go to your Applications folder
3. Drag MonyaCode to the Trash (or right-click and select "Move to Trash")
4. Empty the Trash

### Removing User Data (Optional)

To completely remove all MonyaCode configuration files and data:

1. Open Finder
2. Press `Cmd + Shift + G` to open "Go to Folder"
3. Delete the following directories if they exist:
   - `~/Library/Application Support/MonyaCode`
   - `~/Library/Saved Application State/app.liten.MonyaCode.savedState`
   - `~/Library/Logs/MonyaCode`
   - `~/Library/Caches/app.liten.MonyaCode`

## Linux

### Standard Uninstall

If MonyaCode was installed using the default installation script, run:

```sh
monyacode --uninstall
```

You'll be prompted whether to keep or delete your preferences. After making a
choice, you should see a message that MonyaCode was successfully uninstalled.

If the `monyacode` command is not found in your PATH, try:

```sh
$HOME/.local/bin/monyacode --uninstall
```

or:

```sh
$HOME/.local/monyacode.app/bin/monyacode --uninstall
```

### Package Manager

If you installed MonyaCode using a package manager (such as Flatpak or a
distribution-specific package manager), consult that package manager's
documentation for uninstallation instructions.

### Manual Removal

If the uninstall command fails or MonyaCode was installed to a custom location, you
can manually remove:

- Installation directory: `~/.local/monyacode.app` (or your custom installation path)
- Binary symlink: `~/.local/bin/monyacode`
- Configuration and data: `~/.config/monyacode`

## Windows

### Standard Installation

1. Quit MonyaCode if it's running
2. Open Settings (Windows key + I)
3. Go to "Apps" > "Installed apps" (or "Apps & features" on Windows 10)
4. Search for "MonyaCode"
5. Click the three dots menu next to MonyaCode and select "Uninstall"
6. Follow the prompts to complete the uninstallation

Alternatively, you can:

1. Open the Start menu
2. Right-click on MonyaCode
3. Select "Uninstall"

### Removing User Data (Optional)

To completely remove all MonyaCode configuration files and data:

1. Press `Windows key + R` to open Run
2. Type `%APPDATA%` and press Enter
3. Delete the `MonyaCode` folder if it exists
4. Press `Windows key + R` again, type `%LOCALAPPDATA%` and press Enter
5. Delete the `MonyaCode` folder if it exists

## Troubleshooting

If you encounter issues during uninstallation:

- **macOS/Windows**: Ensure MonyaCode is completely quit before attempting to
  uninstall. Check Activity Manager (macOS) or Task Manager (Windows) for any
  running MonyaCode processes.
- **Linux**: If the uninstall script fails, check the error message and consider
  manual removal of the directories listed above.
- **All platforms**: If you want to start fresh while keeping MonyaCode installed,
  you can delete the configuration directories instead of uninstalling the
  application entirely.

For additional help, see the [Linux-specific documentation](./linux.md).
