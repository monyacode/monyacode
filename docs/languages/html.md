# HTML

HTML support is built into the editor, both syntax highlighting and language
server support.

- [Tree sitter grammar](https://github.com/tree-sitter/tree-sitter-html)

There is language server support available from two different servers:

- [SuperHTML](https://github.com/kristoff-it/superhtml)
- [microsoft/vscode-html-languageservice](https://github.com/microsoft/vscode-html-languageservice)

By default, the Microsoft language server is disabled and SuperHTML enabled. To
switch language server, add the following to your `settings.jsonc`:

```json
{
  "languages": {
    "HTML": {
      "language_servers": ["vscode-html-language-server", "!superhtml", "..."],
    },
  },
}
```

## Formatting

By default MonyaCode uses [Prettier](https://prettier.io/) for formatting HTML.

You can disable `format_on_save` by adding the following to your MonyaCode
`settings.jsonc`:

```json
  "languages": {
    "HTML": {
      "format_on_save": "off",
    }
  }
```

You can still trigger formatting manually with {#kb editor::Format} or by
opening the Command Palette ({#kb command_palette::Toggle}) and selecting
"Format Document".

### LSP Formatting

To use the language server auto-formatting instead of Prettier, add the
following to your MonyaCode settings:

```json
  "languages": {
    "HTML": {
      "formatter": "language_server",
    }
  }
```

You can customize various
[formatting options](https://code.visualstudio.com/docs/languages/html#_formatting)
for `vscode-html-language-server` via your MonyaCode `settings.json`:

```json
  "lsp": {
    "vscode-html-language-server": {
      "settings": {
        "html": {
          "format": {
            // Indent under <html> and <head> (default: false)
            "indentInnerHtml": true,
            // Disable formatting inside <svg> or <script>
            "contentUnformatted": "svg,script",
            // Add an extra newline before <div> and <p>
            "extraLiners": "div,p"
          }
        }
      }
    }
  }
```

## See also

- [CSS](./css.md)
- [JavaScript](./javascript.md)
- [TypeScript](./typescript.md)
