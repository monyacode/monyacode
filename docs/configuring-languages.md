# Configuring supported languages

The language support is built on two main technologies:

1. Tree-sitter: This handles syntax highlighting and structure-based features
   like the outline panel.
2. Language Server Protocol (LSP): This provides semantic features such as code
   completion and diagnostics.

These components work together to provide language capabilities.

## Topics

- Language-specific settings
- File associations
- Working with language servers
- Formatting and linting configuration
- Customizing syntax highlighting and themes
- Advanced language features

See the list of [Supported Languages](./languages.md) for details on specific
configurations. To add support for additional languages, see the guide on how to
create new [Language Extensions](./extensions/languages.md).

## Language-specific Settings

It is possible to override global settings for individual languages. These
custom configurations are defined in the `settings.jsonc` file under the
`languages` key.

Here's an example of language-specific settings:

```json
"languages": {
  "Python": {
    "tab_size": 4,
    "formatter": "language_server",
    "format_on_save": "on"
  },
  "JavaScript": {
    "tab_size": 2,
    "formatter": {
      "external": {
        "command": "prettier",
        "arguments": ["--stdin-filepath", "{buffer_path}"]
      }
    }
  }
}
```

Some of the settings that can be customized include:

- [`tab_size`](./configuring-monyacode.md#tab-size): The number of spaces for each
  indentation level
- [`formatter`](./configuring-monyacode.md#formatter): The tool used for code
  formatting
- [`format_on_save`](./configuring-monyacode.md#format-on-save): Whether to
  automatically format code when saving
- [`enable_language_server`](./configuring-monyacode.md#enable-language-server):
  Toggle language server support
- [`hard_tabs`](./configuring-monyacode.md#hard-tabs): Use tabs instead of spaces for
  indentation
- [`preferred_line_length`](./configuring-monyacode.md#preferred-line-length): The
  recommended maximum line length
- [`soft_wrap`](./configuring-monyacode.md#soft-wrap): How to wrap long lines of code
- [`show_completions_on_input`](./configuring-monyacode.md#show-completions-on-input):
  Whether or not to show completions as you type
- [`show_completion_documentation`](./configuring-monyacode.md#show-completion-documentation):
  Whether to display inline and alongside documentation for items in the
  completions menu
- [`colorize_brackets`](./configuring-monyacode.md#colorize-brackets): Whether to use
  tree-sitter bracket queries to detect and colorize the brackets in the editor
  (also known as "rainbow brackets")

## File Associations

MonyaCode automatically detects file types based on their extensions, but you can
customize these associations.

To set up custom file associations, use the
[`file_types`](./configuring-monyacode.md#file-types) setting in your
`settings.jsonc`:

```json
"file_types": {
  "C++": ["c"],
  "TOML": ["MyLockFile"],
  "Dockerfile": ["Dockerfile*"]
}
```

This configuration tells MonyaCode to:

- Treat `.c` files as C++ instead of C
- Recognize files named "MyLockFile" as TOML
- Apply Dockerfile syntax to any file starting with "Dockerfile"

You can use glob patterns for more flexible matching, allowing you to handle
complex naming conventions in your projects.

## Working with Language Servers

Language servers provide capabilities like auto-completion, go-to-definition,
and real-time error checking.

### What are Language Servers?

Language servers implement the Language Server Protocol (LSP), which
standardizes communication between the editor and language-specific tools. This
allows editors to support advanced features for multiple programming languages
without implementing each feature separately.

Some key features provided by language servers include:

- Code completion
- Error checking and diagnostics
- Code navigation (go to definition, find references)
- Code actions (Rename, extract method)
- Hover information
- Workspace symbol search

### Managing Language Servers

MonyaCode simplifies language server management for users:

1. Suggested extensions: When you open a file with a known file type, you may be
   prompted to install the associated extension. For the installation to work,
   you will need to have [rustup](https://rustup.rs) configured correctly.

2. Storage Location:

   - macOS: `~/Library/Application Support/MonyaCode/languages`
   - Linux: `$XDG_DATA_HOME/monyacode/languages`,
     `$FLATPAK_XDG_DATA_HOME/monyacode/languages`, or
     `$HOME/.local/share/monyacode/languages`

### Choosing Language Servers

Some languages offer multiple language server options. You might have multiple
extensions installed that bundle language servers targeting the same language,
potentially leading to overlapping capabilities. MonyaCode allows you to prioritize
which language servers are used and in what order.

Specify preferences using the `language_servers` setting:

```json
  "languages": {
    "PHP": {
      "language_servers": ["intelephense", "!phpactor", "!phptools", "..."]
    }
  }
```

In this example:

- `intelephense` is set as the primary language server
- `phpactor` is disabled (note the `!` prefix)
- `...` expands to the rest of the language servers that are registered for PHP

### Toolchains

Some language servers need to be configured with a current "toolchain", which is
an installation of a specific version of a programming language compiler or/and
interpreter, which can possibly include a full set of dependencies of a project.

An example of what MonyaCode considers a toolchain is a virtual environment in
Python.

Not all languages in MonyaCode support toolchain discovery and selection, but for
those that do, you can specify the toolchain from a toolchain picker (via
{#action toolchain::Select}). To learn more about toolchains, see
[`toolchains`](./toolchains.md).

### Configuring Language Servers

Custom configuration options for language servers are configured using the `lsp`
section of `settings.jsonc`:

```json
  "lsp": {
    "rust-analyzer": {
      "initialization_options": {
        "check": {
          "command": "clippy"
        }
      }
    }
  }
```

This example configures the Rust Analyzer to use Clippy for additional linting
when saving files.

#### Nested objects

When configuring language server options it's important to use nested objects
rather than dot-delimited strings. This is particularly relevant when working
with more complex configurations. Let's look at a real-world example using the
TypeScript language server:

Suppose you want to configure the following settings for TypeScript:

- Enable strict null checks
- Set the target ECMAScript version to ES2020

Here's how you would structure these settings in `settings.jsonc`:

```json
"lsp": {
  "typescript-language-server": {
    "initialization_options": {
      // These are not supported (VSCode dotted style):
      // "preferences.strictNullChecks": true,
      // "preferences.target": "ES2020"
      //
      // These is correct (nested notation):
      "preferences": {
        "strictNullChecks": true,
        "target": "ES2020"
      },
    }
  }
}
```

#### Possible configuration options

Depending on how a particular language server is implemented, they may depend on
initialization options specified in the LSP.

- [initializationOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#version_3_17_0)

These options are sent during language server startup and require the server to
be restarted for changes to be applied.

For example, rust-analyzer and clangd are configured this way.

```json
  "lsp": {
    "rust-analyzer": {
      "initialization_options": {
        "checkOnSave": false
      }
    }
  }
```

- [Configuration Request](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_configuration)

Most language servers use settings that can be updated without restarting.

```json
"lsp": {
  "tailwindcss-language-server": {
    "settings": {
      "tailwindCSS": {
        "emmetCompletions": true,
      },
    }
  }
}
```

Some language servers allow configuring how the language server binary is
launched. Language servers are automatically downloaded or launched if found in
your path.

TODO: This should be opt in.

If you wish to specify an explicit alternate binary you can specify that in
settings:

```json
  "lsp": {
    "rust-analyzer": {
      "binary": {
        // Whether to fetch the binary from the internet, or attempt to find locally.
        "ignore_system_version": false,
        "path": "/path/to/langserver/bin",
        "arguments": ["--option", "value"],
        "env": {
          "FOO": "BAR"
        }
      }
    }
  }
```

### Enabling or Disabling Language Servers

You can toggle language server support globally or per-language:

```json
  "languages": {
    "Markdown": {
      "enable_language_server": false
    }
  }
```

This disables the language server for Markdown files, which can be useful for
performance in large documentation projects. You can configure this globally in
your `~/.config/monyacode/settings.jsonc` or inside a `.monyacode/settings.jsonc` in your
project directory.

## Formatting and Linting

### Configuring Formatters

MonyaCode supports both built-in and external formatters. See
[`formatter`](./configuring-monyacode.md#formatter) docs for more. Configure
formatters globally or per-language in `settings.jsonc`:

```json
"languages": {
  "JavaScript": {
    "formatter": {
      "external": {
        "command": "prettier",
        "arguments": ["--stdin-filepath", "{buffer_path}"]
      }
    },
    "format_on_save": "on"
  },
  "Rust": {
    "formatter": "language_server",
    "format_on_save": "on"
  }
}
```

This example uses Prettier for JavaScript and the language server's formatter
for Rust, both set to format on save.

To disable formatting for a specific language:

```json
"languages": {
  "Markdown": {
    "format_on_save": "off"
  }
}
```

### Setting Up Linters

Linting is typically handled by language servers.

Many language servers allow you to configure linting rules:

```json
"lsp": {
  "eslint": {
    "settings": {
      "codeActionOnSave": {
        "rules": ["import/order"]
      }
    }
  }
}
```

This configuration sets up ESLint to organize imports on save for JavaScript
files.

To run linter fixes automatically on save:

```json
"languages": {
  "JavaScript": {
    "formatter": {
      "code_action": "source.fixAll.eslint"
    }
  }
}
```

### Integrating Formatting and Linting

Here's an example that uses Prettier for formatting and ESLint for linting
JavaScript files automatically on save:

```json
"languages": {
  "JavaScript": {
    "formatter": [
      {
        "code_action": "source.fixAll.eslint"
      },
      {
        "external": {
          "command": "prettier",
          "arguments": ["--stdin-filepath", "{buffer_path}"]
        }
      }
    ],
    "format_on_save": "on"
  }
}
```

### Troubleshooting

If you encounter issues with formatting or linting:

1. Check the editor log file for error messages (Use the command palette:
   `monyacode: open log`)
2. Ensure external tools (formatters, linters) are correctly installed and in
   your PATH.
3. Verify configurations in both MonyaCode settings and language-specific config
   files (e.g., `.eslintrc`, `.prettierrc`)

## Syntax Highlighting and Themes

### Customizing Syntax Highlighting

MonyaCode uses Tree-sitter grammars for syntax highlighting. Override the default
highlighting using the `theme_overrides` setting.

This example makes comments italic and changes the color of strings:

```json
"theme_overrides": {
  "One Dark": {
    "syntax": {
      "comment": {
        "font_style": "italic"
      },
      "string": {
        "color": "#00AA00"
      }
    }
  }
}
```

### Selecting and Customizing Themes

To change the editor theme:

1. Use the theme selector ({#kb theme_selector::Toggle})
2. Or set it in `settings.jsonc`:

```json
"theme": {
  "mode": "dark",
  "dark": "One Dark",
  "light": "GitHub Light"
}
```

Create custom themes by creating a JSON file in `~/.config/monyacode/themes/`. MonyaCode
will automatically detect and make available any themes in this directory.

### Using Theme Extensions

MonyaCode supports theme extensions. Find the URL for the theme you want to install
and install it from the Extensions panel ({#kb monyacode::Extensions}).

To create your own theme extension, refer to the
[Developing Theme Extensions](./extensions/themes.md) guide.

## Using Language Server Features

### Inlay Hints

Inlay hints provide additional information inline in code, such as parameter
names or inferred types. Configure inlay hints in `settings.jsonc`:

```json
"inlay_hints": {
  "enabled": true,
  "show_type_hints": true,
  "show_parameter_hints": true,
  "show_other_hints": true
}
```

For language-specific inlay hint settings, refer to the documentation for each
language.

### Code Actions

Code actions provide quick fixes and refactoring options. Access code actions
using the `editor: Toggle Code Actions` command or by clicking the lightbulb
icon that appears next to the cursor when actions are available.

### Go To Definition and References

Use these commands to navigate the codebase:

- `editor: Go to Definition` (<kbd>f12|f12</kbd>)
- `editor: Go to Type Definition` (<kbd>cmd-f12|ctrl-f12</kbd>)
- `editor: Find All References` (<kbd>shift-f12|shift-f12</kbd>)

### Rename Symbol

To rename a symbol across a project:

1. Place the cursor on the symbol
2. Use the `editor: Rename Symbol` command (<kbd>f2|f2</kbd>)
3. Enter the new name and press Enter

These features depend on the capabilities of the language server for each
language.

When renaming a symbol that spans multiple files, MonyaCode will open a preview in a
multibuffer. Here you can review all the changes before applying them. To
confirm the rename, simply save the multibuffer. If you decide not to proceed
with the rename, you can undo the changes or close the multibuffer without
saving.

### Hover Information

Use the `editor: Hover` command to display information about the symbol under
the cursor. This often includes type information, documentation, and links to
relevant resources.

### Workspace Symbol Search

The `workspace: Open Symbol` command allows you to search for symbols
(functions, classes, variables) across your entire project. This is useful for
quickly navigating large codebases.

### Code Completion

MonyaCode provides code completion suggestions as you type. You can manually trigger
completion with the `editor: Show Completions` command. Use <kbd>tab|tab</kbd>
or <kbd>enter|enter</kbd> to accept suggestions.

### Diagnostics

Language servers provide real-time diagnostics (errors, warnings, hints) as you
code. View all diagnostics for your project using the `diagnostics: Toggle`
command.
