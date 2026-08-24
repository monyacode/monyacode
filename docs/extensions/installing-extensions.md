# Installing Extensions

Some extensions, particularly extensions that add language support, need to
compile the extension into a Wasm bundle. To do this, MonyaCode uses Rust together
with the `wasm32-wasip2` target to build the Wasm extension on your local
machine.

You can either install these dependencies yourself, or alternatively if you have
[installed Rust via rustup](https://www.rust-lang.org/tools/install), MonyaCode will
try to add the `wasm32-wasip2` target for you.

If you have installed Rust via a package manager but don't have the
`wasm32-wasip2` target installed, installing the extension will not work.

The editor will also need the WASI SDK in order to compile the extension and
tree-sitter grammar for language extensions. If the system-installed version of
`cargo` is able to compile to Wasm, the editor will only download the WASI
sysroot instead.

> **Note:** This is a core difference between Zed and MonyaCode. Zed will download a
> precompiled Wasm bundle from the Zed website when installing an extension, to
> avoid having to build the extension locally.

You can manage your extensions by launching the MonyaCode Extension Gallery.

Press {#kb monyacode::Extensions} , open the command palette and select {#action
monyacode::Extensions} or select "MonyaCode > Extensions" from the menu bar.

Here you can view the extensions that you currently have installed, and you can
also install extensions for your local file system, or via a git URL. To get the
git URL for a Zed extension, go to the Zed extensions website, find the
extension you want to use and copy the "Visit Repository" link.

## Requirements

The exact requirements for compiling Wasm extensions varies by OS and
architecture, so giving exact instructions is very difficult.

You will probably need to install `clang`.

On Chimera Linux, you will need to install `wasm-component-ld` and `wasi-clang`.

On Manjaro, you will need `rust-wasm` and `wasi-compiler-rt`.

## Installing Local Extensions

From the extensions page, click the `Install Local` button (or the {#action
monyacode::InstallExtensionFromFolder} action) and select the directory containing
your extension. You should see the extension begin to install in the status bar.

## Installing Extensions From A Git URL

From the extensions page, click the `Install From URL` button (or the {#action
monyacode::InstallExtensionFromUrl} action) and enter the git url for the extension.
You should see the extension begin to install in the status bar.

> **Note:** The editor must be able to authenticate the git URL, but does not
> support SSH authentication. Using a public URL such as an `http(s)` git URL is
> recommended. Check the logs to debug ({#action monyacode::OpenLog}).

## Installation Location

- On macOS, extensions are installed in
  `~/Library/Application Support/MonyaCode/extensions`.
- On Linux, they are installed in either `$XDG_DATA_HOME/monyacode/extensions` or
  `~/.local/share/monyacode/extensions`.

This directory contains two subdirectories:

- `installed`, which contains the source code for each extension.
- `work` which contains files created by the extension itself, such as
  downloaded language servers.

## Removing extensions

Remove extensions from the Extensions interface.
