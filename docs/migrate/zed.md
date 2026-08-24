# Migrating from Zed to MonyaCode

MonyaCode is a fork of the Zed code editor made roughly around the version `0.217`.
From there, a lot of "features" have been stripped out:

- All telemetry was removed
- All AI-related functionality was removed
- Collaboration and its proprietary server component were removed
- Automatic extension installation was removed
- Auto-updates were removed
- There is no license agreement to accept beyond the GPL open source license

However, some functionality has also been added or changed that is not available
in Zed.

- Integrated documentation viewer
- Various bugfixes
- Rewritten documentation
- Modified defaults
- More themes included

## Configuration

The configuration should be mostly compatible: Try copying your `keymap.json`
and `settings.json` files from `~/.config/zed/` to `~/.config/monyacode/` and
renaming them `keymap.jsonc` and `settings.jsonc`. It might work.

## Extension support

Since Zed extensions in the Zed editor are installed as binary blobs from the
proprietary servers owned by Zed Industries, this installation method has been
removed from MonyaCode. Instead, extensions can be installed using the extension
repository URL directly. Language extensions that need to be compiled to Wasm
require a [rustup](https://rustup.rs) toolchain to do the compilation.

If the extension doesn't work, check the MonyaCode log file using
`cmd-shift-p Open Log` (also available in the Command Palette).

Currently, this functionality is very crude, and as Zed continues to develop it
is likely that extensions that compile to Wasm will stop being compatible with
MonyaCode. Therefore, the plan is to eventually replace the extension system from Zed
with something else, but also to integrate language support for the most popular
languages directly into MonyaCode so that language extensions aren't needed in most
cases.

Note: This is a work in progress. The goal is not to keep up with Zed and to
stay a stripped-down version of the same editor. I am not interested in _Zed_.
The goal is simply to have a good-enough baseline editor which is simple to
install and use, and can replace VS Code as the editor to recommend to
beginners.
