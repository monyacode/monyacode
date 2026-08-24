# Developing Extensions

## Extension Features

Extensions are able to provide the following features to MonyaCode:

- [Languages](../extensions/languages.md)
- [Debuggers](../extensions/debugger-extensions.md)
- [Themes](../extensions/themes.md)
- [Icon Themes](../extensions/icon-themes.md)

## Developing an Extension

Before starting to develop an extension for MonyaCode, be sure to
[install Rust via rustup](https://www.rust-lang.org/tools/install).

Note: If you have installed rustup via [Homebrew](https://brew.sh), you may need
to run `rustup-init` for the command to work properly.

When developing an extension, you can use it in MonyaCode by installing it:

From the extensions page, click the `Install Local` button (or the {#action
monyacode::InstallExtensionFromFolder} action) and select the directory containing
your extension.

If you need to troubleshoot, you can check the MonyaCode.log ({#action
monyacode::OpenLog}) for additional output. For debug output, close and relaunch the
editor with the flag `monyacode --foreground` from the command line which show more
verbose INFO level logging.

## Directory Structure of a MonyaCode Extension

A MonyaCode extension is a Git repository that contains an `extension.toml`. This
file must contain some basic information about the extension:

```toml
id = "my-extension"
name = "My extension"
version = "0.0.1"
schema_version = 1
authors = ["Your Name <you@example.com>"]
description = "My cool extension"
repository = "https://github.com/your-name/my-monyacode-extension"
```

In addition to this, there are several other optional files and directories that
can be used to add functionality to a MonyaCode extension. An example directory
structure of an extension that provides all capabilities is as follows:

```
my-extension/
  extension.toml
  Cargo.toml
  src/
    lib.rs
  languages/
    my-language/
      config.toml
      highlights.scm
  themes/
    my-theme.json
```

## WebAssembly

Procedural parts of extensions are written in Rust and compiled to WebAssembly.
To develop an extension that includes custom code, include a `Cargo.toml` like
this:

```toml
[package]
name = "my-extension"
version = "0.0.1"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.1.0"
```

Use the latest version of the
[`zed_extension_api`](https://crates.io/crates/zed_extension_api) available on
crates.io. Make sure it's still compatible with MonyaCode versions you want to
support.

In the `src/lib.rs` file in your Rust crate you will need to define a struct for
your extension and implement the `Extension` trait, as well as use the
`register_extension!` macro to register your extension:

```rs
use zed_extension_api as zed;

struct MyExtension {
    // ... state
}

impl monyacode::Extension for MyExtension {
    // ...
}

monyacode::register_extension!(MyExtension);
```

> `stdout`/`stderr` is forwarded directly to the MonyaCode process. In order to see
> `println!`/`dbg!` output from your extension, you can start MonyaCode in your
> terminal with a `--foreground` flag.

## Extension License

Extension repositories should include a license. The following licenses are
recommended:

- [GNU GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html)
- [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [BSD 3-Clause](https://opensource.org/license/bsd-3-clause)
- [MIT](https://opensource.org/license/mit)

Your license file should be at the root of your extension repository. Any
filename that has `LICENCE` or `LICENSE` as a prefix (case insensitive).

> Note: This license applies only to your extension code itself. It does not
> apply to any tools your extension may download or interact with, such as
> language servers or other external dependencies. If your repository contains
> both extension code and other projects (like a language server), you don't
> need to relicense those other projects.
