# Using a debugger

> **DISCLAIMER**: This is not documentation for
> [configuring MonyaCode's debugger](../debugger.md). Rather, it is intended to
> provide information on how to use a debugger while developing MonyaCode itself.

## Using MonyaCode's built-in debugger

While the MonyaCode project is open you can open the `New Process Modal` and select
the `Debug` tab. There you can see to debug configurations to debug MonyaCode with,
one for GDB and one for LLDB. Select the configuration you want and MonyaCode will
build and launch the binary.

Note: GDB isn't supported on ARM Macbooks.

## Release build profile considerations

By default, builds using the release profile (release is the profile used for
production builds, i.e. stable) include limited debug info.

This is done by setting the `profile.(release).debug` field in the root
`Cargo.toml` field to `"limited"`.

The official documentation for the `debug` field can be found
[here](https://doc.rust-lang.org/cargo/reference/profiles.html#debug). But the
TLDR is that `"limited"` strips type and variable level debug info.

In release builds, this is done to reduce the binary size, as type and variable
level debug info is not required, and does not impact the usability of generated
stack traces.

However, while the type and variable level debug info is not required for good
stack traces, it is very important for a good experience using debuggers, as
without the type and variable level debug info, the debugger has no way to
resolve local variables, inspect them, format them using pretty-printers, etc.

Therefore, in order to use a debugger to it's fullest extent when debugging a
release build, you must compile a new MonyaCode binary, with full debug info.

The simplest way to do this, is to use the `--config` flag to override the
`debug` field in the root `Cargo.toml` file when running `cargo run` or
`cargo build` like so:

```sh
cargo run --config 'profile.release.debug="full"'
cargo build --config 'profile.release.debug="full"'
```

> If you wish to avoid passing the `--config` flag on every invocation of
> `cargo`. You may also change the section in the
> [root `Cargo.toml`](https://github.com/zed-industries/zed/blob/main/Cargo.toml)
>
> from
>
> ```toml
> [profile.release]
> debug = "limited"
> ```
>
> to
>
> ```toml
> [profile.release]
> debug = "full"
> ```
>
> This will ensure all invocations of `cargo run --release` or
> `cargo build --release` will compile with full debug info.
>
> **WARNING:** Make sure to avoid committing these changes!

## Running MonyaCode with a shell debugger GDB/LLDB

### Background

The recommended way to get rust for MonyaCode development is via
[rustup](https://rustup.rs). For more details, see the documentation for getting
[started on your platform](../development.md).

When rust is installed via rustup, a few additional scripts are installed and
put on your path to assist with debugging binaries compiled with rust:
`rust-gdb` and `rust-lldb`. Learn more about these scripts and why they are
useful on
[a bit of respite](https://michaelwoerister.github.io/2015/03/27/rust-xxdb.html).

In summary, they are simple shell scripts that wrap the standard `gdb` and
`lldb` commands, injecting the relevant commands and flags to enable additional
rust-specific features such as pretty-printers and type information.

In order to use `rust-gdb` or `rust-lldb`, you must have `gdb` or `lldb`
installed on your system. If you don't have them installed, you will need to
install them in a manner appropriate for your platform.

Quoting the article above,

> "The minimum supported debugger versions are GDB 7.7 and LLDB 310. However,
> the general rule is: the newer the better."

It is recommended to install the latest version of `gdb` or `lldb` if possible.

> **Note**: On Windows, `rust-gdb` is not installed by default as `gdb` support
> for Windows is not very stable. Use `lldb` with `rust-lldb` instead.

- [gdb](https://www.gnu.org/software/gdb/)
- [lldb](https://lldb.llvm.org/)

### Usage

Follow the steps above to include full debug info when compiling MonyaCode. Then, run
`rust-gdb` or `rust-lldb` on the compiled MonyaCode binary after building it with
`cargo build`:

```
# for gdb:
rust-gdb target/debug/monyacode

# for lldb:
rust-lldb target/debug/monyacode
```

Alternatively, attach to a running instance of MonyaCode (such as an instance of MonyaCode
started using `cargo run`):

```
rust-gdb -p <pid>
rust-lldb -p <pid>
```

Where `<pid>` is the process ID of the MonyaCode instance you want to attach to.

To get the process ID of a running MonyaCode instance, you can use your systems
process management tools such as `Task Manager` on windows or `Activity Monitor`
on macOS, or from a terminal:

```
# Mac or Linux
ps aux | grep monyacode

# Windows (PowerShell)
Get-Process | Select-Object Id, ProcessName
```

#### Debugging Panics and Crashes

Debuggers can be an excellent tool for debugging the cause of panics and crashes
in all programs, including MonyaCode.

By default, when a process that `gdb` or `lldb` is attached to hits an exception
such as a panic, the debugger will automatically stop at the point of the panic
and allow you to inspect the state of the program.

Most likely, the point at which the debugger stops will be deep in the rust
standard library panic or exception handling code, so you will need to navigate
up the stack trace to find the actual cause of the panic.

This can be accomplished using the `backtrace` command in combination with the
`frame select` command in `lldb`, with similar commands available in `gdb`.

Once the program is stopped, you will not be able to continue execution as you
can before an exception is hit. However, you can jump around to different stack
frames, and inspect the values of variables and expressions within each frame,
which can be very useful in identifying the root cause of the crash.

Read more on [Debugging Crashes](./debugging-crashes.md).
