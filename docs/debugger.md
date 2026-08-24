# Debugger

MonyaCode uses the
[Debug Adapter Protocol (DAP)](https://microsoft.github.io/debug-adapter-protocol/)
to provide debugging functionality across multiple programming languages. DAP is
a standardized protocol that defines how debuggers, editors, and IDEs
communicate with each other. It allows MonyaCode to support various debuggers without
needing to implement language-specific debugging logic. MonyaCode implements the
client side of the protocol, and various _debug adapters_ implement the server
side.

This protocol enables features like setting breakpoints, stepping through code,
inspecting variables, and more, in a consistent manner across different
programming languages and runtime environments.

## Supported Languages

To debug code written in a specific language, MonyaCode needs to find a debug adapter
for that language. Some debug adapters are provided by MonyaCode without additional
setup, and some are provided by
[language extensions](./extensions/debugger-extensions.md). The following
languages currently have debug adapters available:

- [C](./languages/c.md#debugging) (built-in)
- [C++](./languages/cpp.md#debugging) (built-in)
- [Go](./languages/go.md#debugging) (built-in)
- [Java](./languages/java.md#debugging) (provided by extension)
- [JavaScript](./languages/javascript.md#debugging) (built-in)
- [PHP](./languages/php.md#debugging) (built-in)
- [Python](./languages/python.md#debugging) (built-in)
- [Ruby](./languages/ruby.md#debugging) (provided by extension)
- [Rust](./languages/rust.md#debugging) (built-in)
- [Swift](./languages/swift.md#debugging) (provided by extension)
- [TypeScript](./languages/typescript.md#debugging) (built-in)

> If your language isn't listed, you can contribute by
> [adding a debug adapter for it](./extensions/debugger-extensions.md).

Follow those links for language- and adapter-specific information and examples,
or read on for more about MonyaCode's general debugging features that apply to all
adapters.

## Getting Started

For most languages, the fastest way to get started is to run {#action
debugger::Start} ({#kb debugger::Start}). This opens the _new process modal_,
which shows you a contextual list of preconfigured debug tasks for the current
project. Debug tasks are created from tests, entry points (like a `main`
function), and from other sources — consult the documentation for your language
for full information about what's supported.

You can open the same modal by clicking the "plus" button at the top right of
the debug panel.

For languages that don't provide preconfigured debug tasks (this includes C,
C++, and some extension-supported languages), you can define debug
configurations in the `.monyacode/debug.jsonc` file in your project root. This file
should be an array of configuration objects:

```json [debug]
[
  {
    "adapter": "CodeLLDB",
    "label": "First configuration"
    // ...
  },
  {
    "adapter": "Debugpy",
    "label": "Second configuration"
    // ...
  }
]
```

Check the documentation for your language for example configurations covering
typical use-cases. Once you've added configurations to `.monyacode/debug.jsonc`,
they'll appear in the list in the new process modal.

MonyaCode will also load debug configurations from `.vscode/launch.json`, and show
them in the new process modal if no configurations are found in
`.monyacode/debug.jsonc`.

#### Global debug configurations

If you run the same launch profiles across multiple projects, you can store them
once in your user configuration. Invoke {#action monyacode::OpenDebugTasks} from the
command palette to open the global `debug.jsonc` file; MonyaCode creates it next to
your user `settings.jsonc` and keeps it in sync with the debugger UI. The file
lives at:

- **macOS:** `~/Library/Application Support/MonyaCode/debug.jsonc`
- **Linux/BSD:** `$XDG_CONFIG_HOME/monyacode/debug.jsonc` (falls back to
  `~/.config/monyacode/debug.jsonc`)
- **Windows:** `%APPDATA%\MonyaCode\debug.jsonc`

Populate this file with the same array of objects you would place in
`.monyacode/debug.jsonc`. Any scenarios defined there are merged into every
workspace, so your favorite launch presets appear automatically in the "New
Debug Session" dialog.

### Launching & Attaching

MonyaCode debugger offers two ways to debug your program; you can either _launch_ a
new instance of your program or _attach_ to an existing process. Which one you
choose depends on what you are trying to achieve.

When launching a new instance, MonyaCode (and the underlying debug adapter) can often
do a better job at picking up the debug information compared to attaching to an
existing process, since it controls the lifetime of a whole program. Running
unit tests or a debug build of your application is a good use case for
launching.

Compared to launching, attaching to an existing process might seem inferior, but
that's far from truth; there are cases where you cannot afford to restart your
program, because for example, the bug is not reproducible outside of a
production environment or some other circumstances.

## Configuration

MonyaCode requires the `adapter` and `label` fields for all debug tasks. In addition,
MonyaCode will use the `build` field to run any necessary setup steps before the
debugger starts [(see below)](#build-tasks), and can accept a `tcp_connection`
field to connect to an existing process.

All other fields are provided by the debug adapter and can contain
[task variables](./tasks.md#variables). Most adapters support `request`,
`program`, and `cwd`:

```json [debug]
[
  {
    // The label for the debug configuration and used to identify the debug session inside the debug panel & new process modal
    "label": "Example Start debugger config",
    // The debug adapter that MonyaCode should use to debug the program
    "adapter": "Example adapter name",
    // Request:
    //  - launch: MonyaCode will launch the program if specified, or show a debug terminal with the right configuration
    //  - attach: MonyaCode will attach to a running program to debug it, or when the process_id is not specified, will show a process picker (only supported for node currently)
    "request": "launch",
    // The program to debug. This field supports path resolution with ~ or . symbols.
    "program": "path_to_program",
    // cwd: defaults to the current working directory of your project ($MONYACODE_WORKTREE_ROOT)
    "cwd": "$MONYACODE_WORKTREE_ROOT"
  }
]
```

Check your debug adapter's documentation for more information on the fields it
supports.

### Build tasks

MonyaCode allows embedding a MonyaCode task in the `build` field that is run before the
debugger starts. This is useful for setting up the environment or running any
necessary setup steps before the debugger starts.

```json [debug]
[
  {
    "label": "Build Binary",
    "adapter": "CodeLLDB",
    "program": "path_to_program",
    "request": "launch",
    "build": {
      "command": "make",
      "args": ["build", "-j8"]
    }
  }
]
```

Build tasks can also refer to the existing tasks by unsubstituted label:

```json [debug]
[
  {
    "label": "Build Binary",
    "adapter": "CodeLLDB",
    "program": "path_to_program",
    "request": "launch",
    "build": "my build task" // Or "my build task for $MONYACODE_FILE"
  }
]
```

### Automatic scenario creation

Given a MonyaCode task, MonyaCode can automatically create a scenario for you. Automatic
scenario creation also powers our scenario creation from gutter. Automatic
scenario creation is currently supported for Rust, Go, Python, JavaScript, and
TypeScript.

## Breakpoints

To set a breakpoint, simply click next to the line number in the editor gutter.
Breakpoints can be tweaked depending on your needs; to access additional options
of a given breakpoint, right-click on the breakpoint icon in the gutter and
select the desired option. At present, you can:

- Add a log to a breakpoint, which will output a log message whenever that
  breakpoint is hit.
- Make the breakpoint conditional, which will only stop at the breakpoint when
  the condition is met. The syntax for conditions is adapter-specific.
- Add a hit count to a breakpoint, which will only stop at the breakpoint after
  it's hit a certain number of times.
- Disable a breakpoint, which will prevent it from being hit while leaving it
  visible in the gutter.

Some debug adapters (e.g. CodeLLDB and JavaScript) will also _verify_ whether
your breakpoints can be hit; breakpoints that cannot be hit are surfaced more
prominently in the UI.

All breakpoints enabled for a given project are also listed in "Breakpoints"
item in your debugging session UI. From "Breakpoints" item in your UI you can
also manage exception breakpoints. The debug adapter will then stop whenever an
exception of a given kind occurs. Which exception types are supported depends on
the debug adapter.

## Settings

The settings for the debugger are grouped under the `debugger` key in
`settings.jsonc`:

- `dock`: Determines the position of the debug panel in the UI.
- `stepping_granularity`: Determines the stepping granularity.
- `save_breakpoints`: Whether the breakpoints should be reused across MonyaCode
  sessions.
- `button`: Whether to show the debug button in the status bar.
- `timeout`: Time in milliseconds until timeout error when connecting to a TCP
  debug adapter.
- `log_dap_communications`: Whether to log messages between active debug
  adapters and MonyaCode.
- `format_dap_log_messages`: Whether to format DAP messages when adding them to
  the debug adapter logger.

### Dock

- Description: The position of the debug panel in the UI.
- Default: `bottom`
- Setting: debugger.dock

**Options**

1. `left` - The debug panel will be docked to the left side of the UI.
2. `right` - The debug panel will be docked to the right side of the UI.
3. `bottom` - The debug panel will be docked to the bottom of the UI.

```json
"debugger": {
  "dock": "bottom"
},
```

### Stepping granularity

- Description: The Step granularity that the debugger will use
- Default: `line`
- Setting: `debugger.stepping_granularity`

**Options**

1. Statement - The step should allow the program to run until the current
   statement has finished executing. The meaning of a statement is determined by
   the adapter and it may be considered equivalent to a line. For example
   'for(int i = 0; i < 10; i++)' could be considered to have 3 statements 'int i
   = 0', 'i < 10', and 'i++'.

```json
{
  "debugger": {
    "stepping_granularity": "statement",
  },
}
```

2. Line - The step should allow the program to run until the current source line
   has executed.

```json
{
  "debugger": {
    "stepping_granularity": "line",
  },
}
```

3. Instruction - The step should allow one instruction to execute (e.g. one x86
   instruction).

```json
{
  "debugger": {
    "stepping_granularity": "instruction",
  },
}
```

### Save Breakpoints

- Description: Whether the breakpoints should be saved across MonyaCode sessions.
- Default: `true`
- Setting: `debugger.save_breakpoints`

**Options**

`boolean` values

```json
{
  "debugger": {
    "save_breakpoints": true,
  },
}
```

### Button

- Description: Whether the button should be displayed in the debugger toolbar.
- Default: `true`
- Setting: `debugger.show_button`

**Options**

`boolean` values

```json
{
  "debugger": {
    "show_button": true,
  },
}
```

### Timeout

- Description: Time in milliseconds until timeout error when connecting to a TCP
  debug adapter.
- Default: `2000`
- Setting: `debugger.timeout`

**Options**

`integer` values

```json
{
  "debugger": {
    "timeout": 3000,
  },
}
```

### Inline Values

- Description: Whether to enable editor inlay hints showing the values of
  variables in your code during debugging sessions.
- Default: `true`
- Setting: `inlay_hints.show_value_hints`

**Options**

```json
{
  "inlay_hints": {
    "show_value_hints": false,
  },
}
```

Inline value hints can also be toggled from the Editor Controls menu in the
editor toolbar.

### Log Dap Communications

- Description: Whether to log messages between active debug adapters and MonyaCode.
  (Used for DAP development)
- Default: false
- Setting: debugger.log_dap_communications

**Options**

`boolean` values

```json
{
  "debugger": {
    "log_dap_communications": true,
  },
}
```

### Format Dap Log Messages

- Description: Whether to format DAP messages when adding them to the debug
  adapter logger. (Used for DAP development)
- Default: false
- Setting: debugger.format_dap_log_messages

**Options**

`boolean` values

```json
{
  "debugger": {
    "format_dap_log_messages": true,
  },
}
```

### Customizing Debug Adapters

- Description: Custom program path and arguments to override how MonyaCode launches a
  specific debug adapter.
- Default: Adapter-specific
- Setting: `dap.$ADAPTER.binary` and `dap.$ADAPTER.args`

You can pass `binary`, `args`, or both. `binary` should be a path to a _debug
adapter_ (like `lldb-dap`) not a _debugger_ (like `lldb` itself). The `args`
setting overrides any arguments that MonyaCode would otherwise pass to the adapter.

```json
{
  "dap": {
    "CodeLLDB": {
      "binary": "/Users/name/bin/lldb-dap",
      "args": ["--wait-for-debugger"],
    },
  },
}
```

## Theme

The Debugger supports the following theme options:

- `debugger.accent`: Color used to accent breakpoint & breakpoint-related
  symbols
- `editor.debugger_active_line.background`: Background color of active debug
  line

## Troubleshooting

If you're running into problems with the debugger, there are some features you
can use to gather more information about the problem:

- When you have a session running in the debug panel, you can run the {#action
  dev::CopyDebugAdapterArguments} action to copy a JSON blob to the clipboard
  that describes how MonyaCode initialized the session. This is especially useful
  when the session failed to start, and is great context to add if you open a
  GitHub issue.
- You can also use the {#action dev::OpenDebugAdapterLogs} action to see a trace
  of all of MonyaCode's communications with debug adapters during the most recent
  debug sessions.
