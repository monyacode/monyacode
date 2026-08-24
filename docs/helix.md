# Helix Mode

_Work in progress! Not all Helix keybindings are implemented yet._

The Helix mode is an emulation layer that brings Helix-style keybindings and
modal editing to MonyaCode. It builds upon [Vim mode](./vim.md), so much of the core
functionality is shared. Enabling `helix_mode` will also enable `vim_mode`.

For a guide on Vim-related features that are also available in Helix mode,
please refer to the [Vim mode documentation](./vim.md).

For a detailed list of Helix's default keybindings, please visit the
[official Helix documentation](https://docs.helix-editor.com/keymap.html).

## Core differences

Any text object that works with `m i` or `m a` also works with `]` and `[`, so
for example `] (` selects the next pair of parentheses after the cursor.
