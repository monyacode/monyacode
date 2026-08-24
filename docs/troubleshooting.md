# Troubleshooting

This guide covers common troubleshooting techniques for MonyaCode. Sometimes you'll
be able to identify and resolve issues on your own using this information. Other
times, troubleshooting means gathering the right information—logs, profiles, or
reproduction steps—to help developers diagnose and fix the problem.

> **Note**: To open the command palette, use `cmd-shift-p` on macOS or
> `ctrl-shift-p` on Windows / Linux.

## Retrieve MonyaCode and System Information

When reporting issues or seeking help, it's useful to know your MonyaCode version and
system specifications. You can retrieve this information using the following
actions from the command palette:

- {#action monyacode::About}: Find your MonyaCode version number
- {#action monyacode::CopySystemSpecsIntoClipboard}: Populate your clipboard with
  MonyaCode version number, operating system version, and hardware specs

## MonyaCode Log

Often, a good first place to look when troubleshooting any issue in MonyaCode is the
MonyaCode log, which might contain clues about what's going wrong. You can review the
most recent 1000 lines of the log by running the {#action monyacode::OpenLog} action
from the command palette. If you want to view the full file, you can reveal it
in your operating system's native file manager via {#action
monyacode::RevealLogInFileManager} from the command palette.

You'll find the MonyaCode log in the respective location on each operating system:

- macOS: `~/Library/Logs/MonyaCode/MonyaCode.log`
- Windows: `C:\Users\YOU\AppData\Local\MonyaCode\logs\MonyaCode.log`
- Linux: `~/.local/share/monyacode/logs/MonyaCode.log` or `$XDG_DATA_HOME`

> Note: In some cases, it might be useful to monitor the log live, such as when
> [developing a MonyaCode extension](./extensions/developing-extensions.md). Example:
> `tail -f ~/Library/Logs/MonyaCode/MonyaCode.log`

The log may contain enough context to help you debug the issue yourself, or you
may find specific errors that are useful when filing an issue.

## Performance Issues (Profiling)

If you're running into performance issues — such as hitches, hangs, or general
unresponsiveness — having a performance profile will help zero in on what is
getting stuck.

## Minidumps

If MonyaCode is crashing you can set `MONYACODE_GENERATE_MINIDUMPS=1` when running MonyaCode to
get the editor to generate a crash dump file that can be analyzed to debug the
issue. See [debugging-crashes.md](./development/debugging-crashes.md) for more
details.

### macOS

Xcode Instruments (which comes bundled with
[Xcode](https://apps.apple.com/us/app/xcode/id497799835)) is the standard tool
for profiling on macOS.

1. With MonyaCode running, open Instruments
1. Select `Time Profiler` as the profiling template
1. In the `Time Profiler` configuration, set the target to the running MonyaCode
   process
1. Start recording
1. If the performance issue occurs when performing a specific action in MonyaCode,
   perform that action now
1. Stop recording
1. Save the trace file

<!--### Windows-->

<!--### Linux-->

## Startup and Workspace Issues

MonyaCode creates local SQLite databases to persist data relating to its workspace
and your projects. These databases store, for instance, the tabs and panes you
have open in a project, the scroll position of each open file, the list of all
projects you've opened (for the recent projects modal picker), etc. You can find
and explore these databases in the following locations:

- macOS: `~/Library/Application Support/MonyaCode/db`
- Linux and FreeBSD: `~/.local/share/monyacode/db` (or within `XDG_DATA_HOME` or
  `FLATPAK_XDG_DATA_HOME`)
- Windows: `%LOCALAPPDATA%\MonyaCode\db`

The naming convention of these databases takes on the form of
`0-<release_channel>`:

- Stable: `0-stable`
- Dev: `0-dev`

While rare, we've seen a few cases where workspace databases became corrupted,
which prevented MonyaCode from starting. If you're experiencing startup issues, you
can test whether it's workspace-related by temporarily moving the database from
its location, then trying to start MonyaCode again.

> **Note**: Moving the workspace database will cause MonyaCode to create a fresh one.
> Your recent projects, open tabs, etc. will be reset to "factory".

If your issue persists after regenerating the database, please file an issue.

## Language Server Issues

If you're experiencing language-server related issues, such as stale diagnostics
or issues jumping to definitions, restarting the language server via {#action
editor::RestartLanguageServer} from the command palette will often resolve the
issue.

## Licenses and cargo-about

License information for third party dependencies must be correctly provided for
CI to pass.

[`cargo-about`](https://github.com/EmbarkStudios/cargo-about) is used to
automatically comply with open source licenses.

- Is it showing a `no license specified` error for a crate you've created? If
  so, add `publish = false` under `[package]` in your crate's Cargo.toml.
- Is the error `failed to satisfy license requirements` for a dependency? If so,
  first determine what license the project has and whether this system is
  sufficient to comply with this license's requirements. If you're unsure, ask a
  lawyer. Once you've verified that this system is acceptable add the license's
  SPDX identifier to the `accepted` array in `script/licenses/licenses.toml`.
- Is `cargo-about` unable to find the license for a dependency? If so, add a
  clarification field at the end of `script/licenses/licenses.toml`, as
  specified in the
  [cargo-about book](https://embarkstudios.github.io/cargo-about/cli/generate/config.html#crate-configuration).

## Unable to watch for file system changes

By default, MonyaCode uses a method native to the current platform to watch for file
system changes (inotify on Linux, FSEvents on Mac). For some file systems, this
method does not work, for example for NFS or SSHfs mounts, or when connecting to
a Windows remote.

It is possible to use a poll-based file watcher on these file systems instead.
The polling watcher is more resource-intensive than the native file watchers,
but works in more scenarios.

To use the poll watcher, set the `file_watcher` option in the Worktree settings.

For example:

```json
{
  "file_watcher": {
    "mode": "poll",  // "native" | "poll"
    "poll_interval_ms": 2000  // 500-30000ms
  }
}
```
