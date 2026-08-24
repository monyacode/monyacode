# TOML

TOML support is built into the editor.

- Tree-sitter:
  [tree-sitter/tree-sitter-toml](https://github.com/tree-sitter/tree-sitter-toml)

- Crate: [adot-tree-sitter-toml](https://crates.io/crates/adot-tree-sitter-toml)

There is LSP support via [Taplo](https://taplo.tamasfe.dev), but it is disabled
by default. To enable the LSP, add this to your `settings.jsonc`:

```json
{
  "languages": {
    "TOML": {
      "language_servers": ["taplo", "..."],
    },
  },
}
```
