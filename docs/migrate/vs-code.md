# How to Migrate from VS Code to MonyaCode

This guide is for developers who spent serious time in VS Code and want to try
MonyaCode without starting from scratch.

If you're here, you are probably a VS Code user frustrated by the slow,
Electron-based implementation and the increasing reliance on flaky AI tooling
that is creeping into editors like VS Code and Zed, created by US-based
corporations that want to trap you in a subscription-based model of code
editing.

MonyaCode is a fully open source editor without any AI, telemetry or subscriptions,
with support for many of the same languages that VS Code supports.

## Install MonyaCode

MonyaCode is available for macOS, Windows, and Linux.

To install the editor, you will need to build it from source. Fortunately,
building a Rust project is really easy. All you need is https://rustup.rs, which
will help you install the compiler and tooling that compiling MonyaCode requires.

After installation, you can launch MonyaCode in the same way that you usually run
applications on your platform.

## Import Settings from VS Code

During setup, you have the option to import key settings from VS Code. MonyaCode
imports the following settings:

### Settings Imported from VS Code

The following VS Code settings are automatically imported when you use **Import
Settings from VS Code**:

**Editor**

| VS Code Setting                             | MonyaCode Setting                                   |
| ------------------------------------------- | ---------------------------------------------- |
| `editor.fontFamily`                         | `buffer_font_family`                           |
| `editor.fontSize`                           | `buffer_font_size`                             |
| `editor.fontWeight`                         | `buffer_font_weight`                           |
| `editor.tabSize`                            | `tab_size`                                     |
| `editor.insertSpaces`                       | `hard_tabs` (inverted)                         |
| `editor.wordWrap`                           | `soft_wrap`                                    |
| `editor.wordWrapColumn`                     | `preferred_line_length`                        |
| `editor.cursorStyle`                        | `cursor_shape`                                 |
| `editor.cursorBlinking`                     | `cursor_blink`                                 |
| `editor.renderLineHighlight`                | `current_line_highlight`                       |
| `editor.lineNumbers`                        | `gutter.line_numbers`, `relative_line_numbers` |
| `editor.showFoldingControls`                | `gutter.folds`                                 |
| `editor.minimap.enabled`                    | `minimap.show`                                 |
| `editor.minimap.autohide`                   | `minimap.show`                                 |
| `editor.minimap.showSlider`                 | `minimap.thumb`                                |
| `editor.minimap.maxColumn`                  | `minimap.max_width_columns`                    |
| `editor.stickyScroll.enabled`               | `sticky_scroll.enabled`                        |
| `editor.scrollbar.horizontal`               | `scrollbar.axes.horizontal`                    |
| `editor.scrollbar.vertical`                 | `scrollbar.axes.vertical`                      |
| `editor.mouseWheelScrollSensitivity`        | `scroll_sensitivity`                           |
| `editor.fastScrollSensitivity`              | `fast_scroll_sensitivity`                      |
| `editor.cursorSurroundingLines`             | `vertical_scroll_margin`                       |
| `editor.hover.enabled`                      | `hover_popover_enabled`                        |
| `editor.hover.delay`                        | `hover_popover_delay`                          |
| `editor.parameterHints.enabled`             | `auto_signature_help`                          |
| `editor.multiCursorModifier`                | `multi_cursor_modifier`                        |
| `editor.selectionHighlight`                 | `selection_highlight`                          |
| `editor.roundedSelection`                   | `rounded_selection`                            |
| `editor.find.seedSearchStringFromSelection` | `seed_search_query_from_cursor`                |
| `editor.rulers`                             | `wrap_guides`                                  |
| `editor.renderWhitespace`                   | `show_whitespaces`                             |
| `editor.guides.indentation`                 | `indent_guides.enabled`                        |
| `editor.linkedEditing`                      | `linked_edits`                                 |
| `editor.autoSurround`                       | `use_auto_surround`                            |
| `editor.formatOnSave`                       | `format_on_save`                               |
| `editor.formatOnPaste`                      | `auto_indent_on_paste`                         |
| `editor.formatOnType`                       | `use_on_type_format`                           |
| `editor.trimAutoWhitespace`                 | `remove_trailing_whitespace_on_save`           |
| `editor.suggestOnTriggerCharacters`         | `show_completions_on_input`                    |
| `editor.suggest.showWords`                  | `completions.words`                            |
| `editor.inlineSuggest.enabled`              | `show_edit_predictions`                        |

**Files & Workspace**

| VS Code Setting             | MonyaCode Setting                   |
| --------------------------- | ------------------------------ |
| `files.autoSave`            | `autosave`                     |
| `files.autoSaveDelay`       | `autosave.milliseconds`        |
| `files.insertFinalNewline`  | `ensure_final_newline_on_save` |
| `files.associations`        | `file_types`                   |
| `files.watcherExclude`      | `file_scan_exclusions`         |
| `files.watcherInclude`      | `file_scan_inclusions`         |
| `files.simpleDialog.enable` | `use_system_path_prompts`      |
| `search.smartCase`          | `use_smartcase_search`         |
| `search.useIgnoreFiles`     | `search.include_ignored`       |

**Terminal**

| VS Code Setting                       | MonyaCode Setting                        |
| ------------------------------------- | ----------------------------------- |
| `terminal.integrated.fontFamily`      | `terminal.font_family`              |
| `terminal.integrated.fontSize`        | `terminal.font_size`                |
| `terminal.integrated.lineHeight`      | `terminal.line_height`              |
| `terminal.integrated.cursorStyle`     | `terminal.cursor_shape`             |
| `terminal.integrated.cursorBlinking`  | `terminal.blinking`                 |
| `terminal.integrated.copyOnSelection` | `terminal.copy_on_select`           |
| `terminal.integrated.scrollback`      | `terminal.max_scroll_history_lines` |
| `terminal.integrated.macOptionIsMeta` | `terminal.option_as_meta`           |
| `terminal.integrated.{platform}Exec`  | `terminal.shell`                    |
| `terminal.integrated.env.{platform}`  | `terminal.env`                      |

**Tabs & Panels**

| VS Code Setting                                    | MonyaCode Setting                                       |
| -------------------------------------------------- | -------------------------------------------------- |
| `workbench.editor.showTabs`                        | `tab_bar.show`                                     |
| `workbench.editor.showIcons`                       | `tabs.file_icons`                                  |
| `workbench.editor.tabActionLocation`               | `tabs.close_position`                              |
| `workbench.editor.tabActionCloseVisibility`        | `tabs.show_close_button`                           |
| `workbench.editor.focusRecentEditorAfterClose`     | `tabs.activate_on_close`                           |
| `workbench.editor.enablePreview`                   | `preview_tabs.enabled`                             |
| `workbench.editor.enablePreviewFromQuickOpen`      | `preview_tabs.enable_preview_from_file_finder`     |
| `workbench.editor.enablePreviewFromCodeNavigation` | `preview_tabs.enable_preview_from_code_navigation` |
| `workbench.editor.editorActionsLocation`           | `tab_bar.show_tab_bar_buttons`                     |
| `workbench.editor.limit.enabled` / `value`         | `max_tabs`                                         |
| `workbench.editor.restoreViewState`                | `restore_on_file_reopen`                           |
| `workbench.statusBar.visible`                      | `status_bar.show`                                  |

**Project Panel (File Explorer)**

| VS Code Setting                | MonyaCode Setting                        |
| ------------------------------ | ----------------------------------- |
| `explorer.compactFolders`      | `project_panel.auto_fold_dirs`      |
| `explorer.autoReveal`          | `project_panel.auto_reveal_entries` |
| `explorer.excludeGitIgnore`    | `project_panel.hide_gitignore`      |
| `problems.decorations.enabled` | `project_panel.show_diagnostics`    |
| `explorer.decorations.badges`  | `project_panel.git_status`          |

**Git**

| VS Code Setting                      | MonyaCode Setting                                   |
| ------------------------------------ | ---------------------------------------------- |
| `git.enabled`                        | `git_panel.button`                             |
| `git.defaultBranchName`              | `git_panel.fallback_branch_name`               |
| `git.decorations.enabled`            | `git.inline_blame`, `project_panel.git_status` |
| `git.blame.editorDecoration.enabled` | `git.inline_blame.enabled`                     |

**Window & Behavior**

| VS Code Setting                                  | MonyaCode Setting                             |
| ------------------------------------------------ | ---------------------------------------- |
| `window.confirmBeforeClose`                      | `confirm_quit`                           |
| `window.nativeTabs`                              | `use_system_window_tabs`                 |
| `window.closeWhenEmpty`                          | `when_closing_with_no_tabs`              |
| `accessibility.dimUnfocused.enabled` / `opacity` | `active_pane_modifiers.inactive_opacity` |

**Other**

| VS Code Setting      | MonyaCode Setting                                             |
| -------------------- | -------------------------------------------------------- |
| `http.proxy`         | `proxy`                                                  |
| `npm.packageManager` | `node.npm_path`                                          |
| `outline.icons`      | `outline_panel.file_icons`, `outline_panel.folder_icons` |

MonyaCode doesn’t import extensions or keybindings, but this is the fastest way to
get a familiar feel while trying something new. If you skip that step during
setup, you can still import settings manually later via the command palette:

`Cmd+Shift+P → MonyaCode: Import VS Code Settings`

## Set Up Editor Preferences

You can also configure settings manually in the Settings Editor.

To edit your settings:

1. `Cmd+,` to open the Settings Editor.
2. Run `monyacode: open settings` in the Command Palette.

Here’s how common VS Code settings translate:

| VS Code             | MonyaCode               | Notes                          |
| ------------------- | ------------------ | ------------------------------ |
| editor.fontFamily   | buffer_font_family | MonyaCode uses Fira Code by default |
| editor.fontSize     | buffer_font_size   | Set in pixels                  |
| editor.tabSize      | tab_size           | Can override per language      |
| editor.insertSpaces | insert_spaces      | Boolean                        |
| editor.formatOnSave | format_on_save     | Works with formatter enabled   |
| editor.wordWrap     | soft_wrap          | Supports optional wrap column  |

MonyaCode also supports per-project settings. You can find these in the Settings
Editor as well.

## Open or Create a Project

After setup, press `Cmd+O` (`Ctrl+O` on Linux) to open a folder. This becomes
your workspace in MonyaCode. There's no support for multi-root workspaces or
`.code-workspace` files like in VS Code. MonyaCode keeps it simple: one folder, one
workspace.

To start a new project, create a directory using your terminal or file manager,
then open it in MonyaCode. The editor will treat that folder as the root of your
project.

You can also launch MonyaCode from the terminal inside any folder with: `monyacode .`

Once inside a project, use `Cmd+P` to jump between files quickly. `Cmd+Shift+P`
(`Ctrl+Shift+P` on Linux) opens the command palette for running actions / tasks,
toggling settings, or running other actions.

Open buffers appear as tabs across the top. The sidebar shows your file tree and
Git status. Collapse it with `Cmd+B` for a distraction-free view.

## Differences in Keybindings

If you chose the VS Code keymap during onboarding, you're likely good to go, and
most of your shortcuts should already feel familiar. Here’s a quick reference
guide for how our keybindings compare to what you’re used to coming from VS
Code.

### Common Shared Keybindings (MonyaCode <> VS Code)

| Action                      | Shortcut               |
| --------------------------- | ---------------------- |
| Find files                  | `Cmd + P`              |
| Run a command               | `Cmd + Shift + P`      |
| Search text (project-wide)  | `Cmd + Shift + F`      |
| Find symbols (project-wide) | `Cmd + T`              |
| Find symbols (file-wide)    | `Cmd + Shift + O`      |
| Toggle left dock            | `Cmd + B`              |
| Toggle bottom dock          | `Cmd + J`              |
| Open terminal               | `Ctrl + ~`             |
| Open file tree explorer     | `Cmd + Shift + E`      |
| Close current buffer        | `Cmd + W`              |
| Close whole project         | `Cmd + Shift + W`      |
| Refactor: rename symbol     | `F2`                   |
| Change theme                | `Cmd + K, Cmd + T`     |
| Wrap text                   | `Opt + Z`              |
| Navigate open tabs          | `Cmd + Opt + Arrow`    |
| Syntactic fold / unfold     | `Cmd + Opt + {` or `}` |

### Different Keybindings (MonyaCode <> VS Code)

| Action              | VS Code               | MonyaCode                   |
| ------------------- | --------------------- | ---------------------- |
| Open recent project | `Ctrl + R`            | `Cmd + Opt + O`        |
| Move lines up/down  | `Opt + Up/Down`       | `Cmd + Ctrl + Up/Down` |
| Split panes         | `Cmd + \`             | `Cmd + K, Arrow Keys`  |
| Expand Selection    | `Shift + Alt + Right` | `Opt + Up`             |

### Unique to MonyaCode

| Action              | Shortcut                     | Notes                                            |
| ------------------- | ---------------------------- | ------------------------------------------------ |
| Toggle right dock   | `Cmd + R` or `Cmd + Alt + B` |                                                  |
| Syntactic selection | `Opt + Up/Down`              | Selects code by structure (e.g., inside braces). |

### How to Customize Keybindings

To edit your keybindings:

- Open the command palette (`Cmd+Shift+P`)
- Run `MonyaCode: Open Keymap Editor`

This opens a list of all available bindings. You can override individual
shortcuts, remove conflicts, or build a layout that works better for your setup.

MonyaCode also supports chords (multi-key sequences) like `Cmd+K Cmd+C`, like VS Code
does.

## Differences in User Interfaces

### No Workspace

VS Code uses a dedicated Workspace concept, with multi-root folders,
`.code-workspace` files, and a clear distinction between “a window” and “a
workspace.” MonyaCode simplifies this model.

In MonyaCode:

- There is no workspace file format. Opening a folder is your project context.

- MonyaCode does not support multi-root workspaces. You can only open one folder at a
  time in a window.

- Most project-level behavior is scoped to the folder you open. Search, Git
  integration, tasks, and environment detection all treat the opened directory
  as the project root.

- Per-project settings are optional. You can add a `.monyacode/settings.json` file
  inside a project to override global settings, but MonyaCode does not use
  `.code-workspace` files and won’t import them.

- You can start from a single file or an empty window. MonyaCode doesn’t require you
  to open a folder to begin editing.

The result is a simpler model: Open a folder → work inside that folder → no
additional workspace layer.

### Navigating in a Project

In VS Code, the standard entry point is opening a folder. From there, the
left-hand sidebar is central to your navigation. MonyaCode takes a different
approach:

- You can still open folders, but you don’t need to. Opening a single file or
  even starting with an empty workspace is valid.
- The Command Palette (`Cmd+Shift+P`) and File Finder (`Cmd+P`) are your primary
  navigation tools. The File Finder searches across the entire workspace
  instantly; files, symbols and commands.
- Instead of a persistent sidebar, MonyaCode encourages you to:
  - Fuzzy-find files by name (`Cmd+P`)
  - Jump directly to symbols (`Cmd+Shift+O`)
  - Use split panes and tabs for context, rather than keeping a large file tree
    open (though you can do this with the Project Panel if you prefer).

The UI is intentionally minimal. Panels slide in only when needed, then get out
of your way. The focus is on flowing between code instead of managing panes.

### Extensions vs. Marketplace

MonyaCode does not offer as many extensions as VS Code. The available extensions are
focused on language support, themes, syntax highlighting, and other core editing
enhancements.

However there are several features that typically require extensions in VS Code
which we built directly into MonyaCode:

- Vim modal editing mode (as well as a Helix mode)
- Built-in terminal panel
- Project-wide fuzzy search
- Task runner with JSON config
- Inline diagnostics and code actions via LSP

You won’t find one-to-one replacements for every VS Code extension, especially
if you rely on tools for DevOps, containers, or test runners. MonyaCode's extension
ecosystem is still growing, and the catalog is smaller by design.

### Advanced Config and Productivity Tweaks

MonyaCode exposes advanced settings for power users who want to fine-tune their
environment.

Here are a few useful tweaks:

**Format on Save:**

```json
"format_on_save": "on"
```

**Enable direnv support:**

```json
"load_direnv": "shell_hook"
```

**Custom Tasks**: Define build or run commands in your `tasks.jsonc` (accessed
via command palette: `monyacode: open tasks`):

```json
[
  {
    "label": "build",
    "command": "cargo build"
  }
]
```

**Bring over custom snippets** Copy your VS Code snippet JSON directly into
MonyaCode's snippets folder (`monyacode: configure snippets`).
