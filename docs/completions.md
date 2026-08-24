# Completions

"Code Completions" provided by Language Servers (LSPs) automatically installed
by MonyaCode or via [MonyaCode Language Extensions](./languages.md).

## Language Server Code Completions {#code-completions}

When there is an appropriate language server available, MonyaCode will provide
completions of variable names, functions, and other symbols in the current file.
You can disable these by adding the following to your MonyaCode `settings.jsonc`
file:

```json
"show_completions_on_input": false
```

You can manually trigger completions with `ctrl-space` or by triggering the
`editor::ShowCompletions` action from the command palette.

For more information, see:

- [Configuring Supported Languages](./configuring-languages.md)
- [List of MonyaCode Supported Languages](./languages.md)
