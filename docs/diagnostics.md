# Diagnostics

MonyaCode gets its diagnostics from the language servers and supports both push and
pull variants of the LSP which should make it compatible with all existing
language servers.

# Regular diagnostics

By default, all diagnostics are displayed as underlined text in the editor and
the scrollbar.

Editor diagnostics can be filtered using the editor setting:

```json
"diagnostics_max_severity": null
```

Possible values: `"off"`, `"error"`, `"warning"`, `"info"`, `"hint"`, `null`
(default, all diagnostics)

The scrollbar diagnostics are configured using

```json
"scrollbar": {
  "diagnostics": "all",
}
```

Possible values: `"none"`, `"error"`, `"warning"`, `"information"`, `"all"`
(default)

The diagnostics can be hovered to display a tooltip with full, rendered
diagnostic message. Or, `editor::GoToDiagnostic` and
`editor::GoToPreviousDiagnostic` could be used to navigate between diagnostics
in the editor, showing a popover for the currently active diagnostic.

# Inline diagnostics (Error lens)

It's possible to display diagnostics as a lens to the right of the code. This is
disabled by default, but can either be temporarily turned on (or off) using the
editor menu, or permanently, using the

```json
"diagnostics": {
  "inline": {
    "enabled": true,
    "max_severity": null, // same values as the `diagnostics_max_severity` from the editor settings
  }
}
```

# Other UI places

## Project Panel

The project panel can have its entries coloured based on the severity of the
diagnostics in the file:

```json
"project_panel": {
  "show_diagnostics": "all",
}
```

Possible values: `"off"`, `"errors"`, `"all"` (default)

## Editor tabs

Editor tabs can be coloured in the same way as project panel entries:

```json
"tabs": {
  "show_diagnostics": "off",
}
```

Possible values: `"off"` (default), `"errors"`, `"all"`
