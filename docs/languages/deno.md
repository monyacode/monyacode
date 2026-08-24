# Deno

Deno support is available through the
[Deno extension](https://github.com/zed-extensions/deno).

- Language server:
  [Deno Language Server](https://docs.deno.com/runtime/manual/advanced/language_server/overview/)

## Deno Configuration

To use the Deno Language Server with TypeScript and TSX files, you will likely
wish to disable the default language servers and enable deno by adding the
following to your `settings.jsonc`:

```json
{
  "lsp": {
    "deno": {
      "settings": {
        "deno": {
          "enable": true,
        },
      },
    },
  },
  "languages": {
    "JavaScript": {
      "language_servers": [
        "deno",
        "!typescript-language-server",
        "!vtsls",
        "!eslint",
      ],
      "formatter": "language_server",
    },
    "TypeScript": {
      "language_servers": [
        "deno",
        "!typescript-language-server",
        "!vtsls",
        "!eslint",
      ],
      "formatter": "language_server",
    },
    "TSX": {
      "language_servers": [
        "deno",
        "!typescript-language-server",
        "!vtsls",
        "!eslint",
      ],
      "formatter": "language_server",
    },
  },
}
```

See [Configuring supported languages](../configuring-languages.md) in the MonyaCode
documentation for more information.

## Configuration completion

To get completions for `deno.json` or `package.json` you can add the following
to your `settings.jsonc`: (More info here ./json.md)

```json
"lsp": {
    "json-language-server": {
      "settings": {
        "json": {
          "schemas": [
            {
              "fileMatch": [
                "deno.json",
                "deno.jsonc"
              ],
              "url": "https://raw.githubusercontent.com/denoland/deno/refs/heads/main/cli/schemas/config-file.v1.json"
            },
            {
              "fileMatch": [
                "package.json"
              ],
              "url": "https://www.schemastore.org/package"
            }
          ]
        }
      }
    }
  }
```

## DAP support

To debug deno programs, add this to `.monyacode/debug.jsonc`

```json [debug]
[
  {
    "adapter": "JavaScript",
    "label": "Deno",
    "request": "launch",
    "type": "pwa-node",
    "cwd": "$MONYACODE_WORKTREE_ROOT",
    "program": "$MONYACODE_FILE",
    "runtimeExecutable": "deno",
    "runtimeArgs": ["run", "--allow-all", "--inspect-wait"],
    "attachSimplePort": 9229
  }
]
```

## Runnable support

To run deno tasks like tests from the ui, add this to `.monyacode/tasks.jsonc`

```json [tasks]
[
  {
    "label": "deno test",
    "command": "deno test -A --filter '/^$MONYACODE_CUSTOM_DENO_TEST_NAME$/' '$MONYACODE_FILE'",
    "tags": ["js-test"]
  }
]
```

## See also:

- [TypeScript](./typescript.md)
- [JavaScript](./javascript.md)
