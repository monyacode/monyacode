# Multibuffers

Using multibuffers, it's possible to edit multiple files simultaneously from a
single view.

Multibuffers work well together with multiple cursors.

## Editing in a multibuffer

Editing a multibuffer is the same as editing a normal file. Changes will be
reflected in the open copies of the files in the rest of the editor.

Save all files with {#action editor::Save} (bound to `cmd-s` on macOS, `ctrl-s`
on Windows/Linux, or `:w` in Vim mode).

When in a multibuffer, it is often useful to use multiple cursors to edit every
file simultaneously. To edit multiple instances at once, select them with the
mouse (`option-click` on macOS, `alt-click` on Window/Linux) or the keyboard.
`cmd-d` on macOS, `ctrl-d` on Windows/Linux, or `gl` in Vim mode will select the
next match of the word under the cursor.

To edit all matches you can select them by running the {#action
editor::SelectAllMatches} command (`cmd-shift-l` on macOS, `ctrl-shift-l` on
Windows/Linux, or `g a` in Vim mode).

## Navigating to the Source File

Click on any of the divider lines between excerpts or place the cursor in an
excerpt, and run the {#action editor::OpenExcerpts} command. Note: If multiple
cursors are being used, the command will open the source file positioned under
each cursor within the multibuffer.

Double-clicking excerpts with the mouse to open the source file is disabled by
default, and can be enabled using the setting:
`"double_click_in_multibuffer": "open"`.

## Project search

Run {#action pane::ToggleSearch} (`cmd-shift-f` on macOS, `ctrl-shift-f` on
Windows/Linux, or `g/` in Vim mode) to search across the whole project. After
the search has completed, the results will be shown in a new multibuffer. There
will be one excerpt for each matching line across the whole project.

## Diagnostics

If an appropriate language server is installed, the diagnostics pane can display
all errors across the project. Open it by clicking on the icon in the status
bar, or run the {#action diagnostics::Deploy} command` ('cmd-shift-m` on macOS,
`ctrl-shift-m` on Windows/Linux, or `:clist` in Vim mode).

## Find References

With a language server installed, find all references to the symbol under the
cursor using the {#action editor::FindReferences} command (`cmd-click` on macOS,
`ctrl-click` on Windows/Linux, or `g A` in Vim mode.

Depending on the language server, commands like {#action editor::GoToDefinition}
and {#action editor::GoToTypeDefinition} will also open a multibuffer if there
are multiple possible definitions.
