# Outline Panel

In addition to the modal outline (`cmd-shift-o`), MonyaCode has the _outline panel_.
The outline panel can be deployed via `cmd-shift-b` ({#action
outline_panel::ToggleFocus} via the command palette), or by clicking the
`Outline Panel` button in the status bar.

When viewing a "singleton" buffer (i.e., a single file on a tab), the outline
panel works similarly to that of the outline modal－it displays the outline of
the current buffer's symbols, as reported by
[tree-sitter](https://tree-sitter.github.io/tree-sitter/). Clicking on an entry
jumps to the associated section in the file. The outline view will also
automatically scroll to the section associated with the current cursor position
within the file.

## Project Search Results

Get an overview of search results across the project.

## Project Diagnostics

View a summary of all errors and warnings reported by the language server.

## Find All References

Quickly navigate through all references when using the {#action
editor::FindAllReferences} action.

The outline view provides a way to quickly navigate to specific parts of the
code to help maintain context when working with large result sets in
multi-buffers.
