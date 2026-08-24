# How to Set Up Python in MonyaCode

Python support is available natively in MonyaCode.

- Tree-sitter:
  [tree-sitter-python](https://github.com/zed-industries/tree-sitter-python)
- Language Servers:
  - [DetachHead/basedpyright](https://github.com/DetachHead/basedpyright)
  - [astral-sh/ruff](https://github.com/astral-sh/ruff)
  - [astral-sh/ty](https://github.com/astral-sh/ty)
  - [microsoft/pyright](https://github.com/microsoft/pyright)
  - [python-lsp/python-lsp-server](https://github.com/python-lsp/python-lsp-server)
    (PyLSP)
- Debug Adapter: [debugpy](https://github.com/microsoft/debugpy)

## Install Python

You'll need both MonyaCode and Python installed before you can begin.

### Step 1: Install Python

MonyaCode does not bundle a Python runtime, so you’ll need to install one yourself.
Choose one of the following options:

- uv (recommended)

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

To learn more, visit
[Astral’s installation guide](https://docs.astral.sh/uv/getting-started/installation/).

- Homebrew:

```bash
brew install python
```

- Python.org installer: Download the latest version from
  [python.org/downloads](https://python.org/downloads).

### Step 2: Verify Python Installation

Confirm Python is installed and available in your shell:

```bash
python3 --version
```

You should see an output like `Python 3.x.x`.

## Open Your First Python Project in MonyaCode

Once MonyaCode and Python are installed, open a folder containing Python code to
start working.

### Step 1: Launch MonyaCode with a Python Project

Open MonyaCode. From the menu bar, choose File > Open Folder, or launch from the
terminal:

```bash
monyacode path/to/your/project
```

MonyaCode will recognize `.py` files automatically using its native
tree-sitter-python parser, with no plugins or manual setup required.

### Step 2: Use the Integrated Terminal (Optional)

MonyaCode includes an integrated terminal, accessible from the bottom panel. If MonyaCode
detects that your project is using a
[virtual environment](#virtual-environments), it will be activated automatically
in newly-created terminals. You can configure this behavior with the
[`detect_venv`](../configuring-monyacode.md#terminal-detect_venv) setting.

## Configure Python Language Servers in MonyaCode

MonyaCode provides several Python language servers out of the box. By default,
[basedpyright](https://github.com/DetachHead/basedpyright) is the primary
language server, and [Ruff](https://github.com/astral-sh/ruff) is used for
formatting and linting.

Other built-in language servers are:

- [Ty](https://docs.astral.sh/ty/), an up-and-coming language server from
  Astral, built for speed.
- [Pyright](https://github.com/microsoft/pyright), the basis for basedpyright.
- [PyLSP](https://github.com/python-lsp/python-lsp-server), a plugin-based
  language server that integrates with tools like `pycodestyle`, `autopep8`, and
  `yapf`.

These are disabled by default, but can be enabled in your settings. For example:

```json
{
  "languages": {
    "Python": {
      "language_servers": [
        // Disable basedpyright and enable Ty, and otherwise
        // use the default configuration.
        "ty",
        "!basedpyright",
        "...",
      ],
    },
  },
}
```

See:
[Working with Language Servers](../configuring-languages.md#working-with-language-servers)
for more information about how to enable and disable language servers.

### Basedpyright

[basedpyright](https://docs.basedpyright.com/latest/) is the primary Python
language server in MonyaCode beginning with MonyaCode v0.204.0. It provides core language
server functionality like navigation (go to definition/find all references) and
type checking. Compared to Pyright, it adds support for additional language
server features (like inlay hints) and checking rules.

Note that while basedpyright in isolation defaults to the `recommended`
[type-checking mode](https://docs.basedpyright.com/latest/benefits-over-pyright/better-defaults/#typecheckingmode),
MonyaCode configures it to use the less-strict `standard` mode by default, which
matches the behavior of Pyright. You can set the type-checking mode for your
project using the `typeCheckingMode` setting in `pyrightconfig.json` or
`pyproject.toml`, which will override MonyaCode's default. Read on more for more
details about how to configure basedpyright.

#### Basedpyright Configuration

basedpyright reads configuration options from two different kinds of sources:

- Language server settings ("workspace configuration"), which must be configured
  per-editor (using `settings.json` in MonyaCode's case) but apply to all projects
  opened in that editor
- Configuration files (`pyrightconfig.json`, `pyproject.toml`), which are
  editor-independent but specific to the project where they are placed

As a rule of thumb, options that are only relevant when using basedpyright from
an editor must be set in language server settings, and options that are relevant
even if you're running it
[as a command-line tool](https://docs.basedpyright.com/latest/configuration/command-line/)
must be set in configuration files. Settings related to inlay hints are examples
of the first category, and the
[diagnostic category](https://docs.basedpyright.com/latest/configuration/config-files/#diagnostic-categories)
settings are examples of the second category.

Examples of both kinds of configuration are provided below. Refer to the
basedpyright documentation on
[language server settings](https://docs.basedpyright.com/latest/configuration/language-server-settings/)
and
[configuration files](https://docs.basedpyright.com/latest/configuration/config-files/)
for comprehensive lists of available options.

##### Language server settings

Language server settings for basedpyright in MonyaCode can be set in the `lsp`
section of your `settings.json`.

For example, in order to:

- diagnose all files in the workspace instead of the only open files default
- disable inlay hints on function arguments

You can use the following configuration:

```json
{
  "lsp": {
    "basedpyright": {
      "settings": {
        "basedpyright.analysis": {
          "diagnosticMode": "workspace",
          "inlayHints": {
            "callArgumentNames": false,
          },
        },
      },
    },
  },
}
```

##### Configuration files

basedpyright reads project-specific configuration from the `pyrightconfig.json`
configuration file and from the `[tool.basedpyright]` and `[tool.pyright]`
sections of `pyproject.toml` manifests. `pyrightconfig.json` overrides
`pyproject.toml` if configuration is present in both places.

Here's an example `pyrightconfig.json` file that configures basedpyright to use
the `strict` type-checking mode and not to issue diagnostics for any files in
`__pycache__` directories:

```json
{
  "typeCheckingMode": "strict",
  "ignore": ["**/__pycache__"],
}
```

### PyLSP

[python-lsp-server](https://github.com/python-lsp/python-lsp-server/), more
commonly known as PyLSP, by default integrates with a number of external tools
(autopep8, mccabe, pycodestyle, yapf) while others are optional and must be
explicitly enabled and configured (flake8, pylint).

See
[Python Language Server Configuration](https://github.com/python-lsp/python-lsp-server/blob/develop/CONFIGURATION.md)
for more.

## Virtual Environments

[Virtual environments](https://docs.python.org/3/library/venv.html) are a useful
tool for fixing a Python version and set of dependencies for a specific project,
in a way that's isolated from other projects on the same machine. MonyaCode has
built-in support for discovering, configuring, and activating virtual
environments, based on the language-agnostic concept of a
[toolchain](../toolchains.md).

Note that if you have a global Python installation, it is also counted as a
toolchain for MonyaCode's purposes.

### Create a Virtual Environment

If your project doesn't have a virtual environment set up already, you can
create one as follows:

```bash
python3 -m venv .venv
```

Alternatively, if you're using `uv`, running `uv sync` will create a virtual
environment the first time you run it.

### How MonyaCode Uses Python Toolchains

MonyaCode uses the selected Python toolchain for your project in the following ways:

- Built-in language servers will be automatically configured with the path to
  the toolchain's Python interpreter and, if applicable, virtual environment.
  This is important so that they can resolve dependencies. (Note that language
  servers provided by extensions can't be automatically configured like this
  currently.)
- Python tasks (such as pytest tests) will be run using the toolchain's Python
  interpreter.
- If the toolchain is a virtual environment, the environment's activation script
  will be run automatically when you launch a new shell in MonyaCode's integrated
  terminal, giving you convenient access to the selected Python interpreter and
  dependency set.
- If a built-in language server is installed in the active virtual environment,
  that binary will be used instead of MonyaCode's private automatically-installed
  binary. This also applies to debugpy.

### Selecting a Toolchain

For most projects, MonyaCode will automatically select the right Python toolchain. In
complex projects with multiple virtual environments, it might be necessary to
override this selection. You can use the
[toolchain selector](../toolchains.md#selecting-toolchains) to pick a toolchain
from the list discovered by MonyaCode, or
[specify the path to a toolchain manually](../toolchains.md#adding-toolchains-manually)
if it's not on the list.

## Code Formatting & Linting

MonyaCode provides the [Ruff](https://docs.astral.sh/ruff/) formatter and linter for
Python code. (Specifically, MonyaCode runs Ruff as an LSP server using the
`ruff server` subcommand.) Both formatting and linting are enabled by default,
including format-on-save.

### Configuring formatting

You can disable format-on-save for Python files in your `settings.json`:

```json
{
  "languages": {
    "Python": {
      "format_on_save": "off",
    },
  },
}
```

Alternatively, you can use the `black` command-line tool for Python formatting,
while keeping Ruff enabled for linting:

```json
{
  "languages": {
    "Python": {
      "formatter": {
        "external": {
          "command": "black",
          "arguments": ["--stdin-filename", "{buffer_path}", "-"],
        },
      },
      // Or use `"formatter": null` to disable formatting entirely.
    },
  },
}
```

### Configuring Ruff

Like basedpyright, Ruff reads options from both MonyaCode's language server settings
and configuration files (`ruff.toml`) when used in MonyaCode. Unlike basedpyright,
_all_ options can be configured in either of these locations, so the choice of
where to put your Ruff configuration comes down to whether you want it to be
shared between projects but specific to MonyaCode (in which case you should use
language server settings), or specific to one project but common to all Ruff
invocations (in which case you should use `ruff.toml`).

Here's an example of using language server settings in MonyaCode's `settings.json` to
disable all Ruff lints in MonyaCode (while still using Ruff as a formatter):

```json
{
  "lsp": {
    "ruff": {
      "initialization_options": {
        "settings": {
          "exclude": ["*"],
        },
      },
    },
  },
}
```

And here's an example `ruff.toml` with linting and formatting options, adapted
from the Ruff documentation:

```toml
[lint]
# Avoid enforcing line-length violations (`E501`)
ignore = ["E501"]

[format]
# Use single quotes when formatting.
quote-style = "single"
```

For more details, refer to the Ruff documentation about
[configuration files](https://docs.astral.sh/ruff/configuration/) and
[language server settings](https://docs.astral.sh/ruff/editors/settings/), and
the [list of options](https://docs.astral.sh/ruff/settings/).

### Embedded Language Highlighting

MonyaCode supports syntax highlighting for code embedded in Python strings by adding
a comment with the language name.

```python
# sql
query = "SELECT * FROM users"

#sql
query = """
    SELECT *
    FROM users
"""

result = func( #sql
    "SELECT * FROM users"
)
```

## Debugging

MonyaCode supports Python debugging through the `debugpy` adapter. You can start with
no configuration or define custom launch profiles in `.monyacode/debug.json`.

### Start Debugging with No Setup

MonyaCode can automatically detect debuggable Python entry points. Press F4 (or run
debugger: start from the Command Palette) to see available options for your
current project. This works for:

- Python scripts
- Modules
- pytest tests

MonyaCode uses `debugpy` under the hood, but no manual adapter configuration is
required.

### Define Custom Debug Configurations

For reusable setups, create a `.monyacode/debug.jsonc` file in your project root.
This gives you more control over how MonyaCode runs and debugs your code.

- [debugpy configuration documentation](https://github.com/microsoft/debugpy/wiki/Debug-configuration-settings#launchattach-settings)

#### Debug Active File

```json [debug]
[
  {
    "label": "Python Active File",
    "adapter": "Debugpy",
    "program": "$MONYACODE_FILE",
    "request": "launch"
  }
]
```

This runs the file currently open in the editor.

#### Debug a Flask App

For projects using Flask, you can define a full launch configuration:

```
.venv/
app/
  init.py
  main.py
  routes.py
templates/
  index.html
static/
  style.css
requirements.txt
```

…the following configuration can be used:

```json [debug]
[
  {
    "label": "Python: Flask",
    "adapter": "Debugpy",
    "request": "launch",
    "module": "app",
    "cwd": "$MONYACODE_WORKTREE_ROOT",
    "env": {
      "FLASK_APP": "app",
      "FLASK_DEBUG": "1"
    },
    "args": [
      "run",
      "--reload", // Enables Flask reloader that watches for file changes
      "--debugger" // Enables Flask debugger
    ],
    "autoReload": {
      "enable": true
    },
    "jinja": true,
    "justMyCode": true
  }
]
```

These can be combined to tailor the experience for web servers, test runners, or
custom scripts.

## Troubleshoot and Maintain a Productive Python Setup

MonyaCode is designed to minimize configuration overhead, but occasional issues can
still arise—especially around environments, language servers, or tooling. Here's
how to keep your Python setup working smoothly.

### Resolve Language Server Startup Issues

If a language server isn't responding or features like diagnostics or
autocomplete aren't available:

- Check your MonyaCode log (using the {#action monyacode::OpenLog} action) for errors
  related to the language server you're trying to use. This is where you're
  likely to find useful information if the language server failed to start up at
  all.
- Use the language server logs view to understand the lifecycle of the affected
  language server. You can access this view using the {#action
  dev::OpenLanguageServerLogs} action, or by clicking the lightning bolt icon in
  the status bar and selecting your language server. The most useful pieces of
  data in this view are:
  - "Server Logs", which shows any errors printed by the language server
  - "Server Info", which shows details about how the language server was started
- Verify your `settings.json` or `pyrightconfig.json` is syntactically correct.
- Restart MonyaCode to reinitialize language server connections, or try restarting
  the language server using the {#action editor::RestartLanguageServer}

If the language server is failing to resolve imports, and you're using a virtual
environment, make sure that the right environment is chosen in the selector. You
can use "Server Info" view to confirm which virtual environment MonyaCode is sending
to the language server. Look for the `* Configuration` section at the end.
