# Nix

Nix support is built into the editor.

## Choosing a language server

There is support for two different language servers: `nil` and `nixd`.

To only enable `nil`:

```json
  "languages": {
    "Nix": {
      "language_servers": ["nil", "!nixd", "..."]
    },
  }
```

To only enable `nixd`:

```json
  "languages": {
    "Nix": {
      "language_servers": ["!nil", "nixd", "..."]
    },
  }
```

## Configuration

Language server options can be set in the `settings.jsonc` file. See the
[Configuring Languages](../configuring-languages.md) documentation for more
information.

### nixd

For more details, see the
[nixd documentation](https://github.com/nix-community/nixd/blob/main/nixd/docs/configuration.md).

```json
{
  "lsp": {
    "nixd": {
      "settings": {
        "diagnostic": {
          "suppress": [ "sema-extra-with" ]
        }
      }
    }
  }
}
```

### nil

For more details, see the
[nil documentation](https://github.com/oxalica/nil/blob/main/docs/configuration.md).

```json
{
  "lsp": {
    "nil": {
      "settings": {
         "diagnostics": {
          "ignored": [ "unused_binding" ]
        }
      }
    }
  }
}
```
