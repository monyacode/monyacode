<div align="center">

![MonyaCode logo](./monyacode-logo.png)

# MonyaCode

A fast, native Rust code editor and IDE for macOS, Linux, *BSD, and Windows.
MonyaCode is designed to be reliable, distraction-free, hackable, and usable
out of the box.

</div>

## Features

- Native cross-platform performance
- Remote development over SSH
- Integrated Git and debugger support
- Jupyter Notebook and REPL support
- Vim and Helix modal editing modes
- Extensive language and LSP support
- Themes, keymaps, and extension support
- No telemetry, subscriptions, or proprietary server components
- No involuntary automatic updates

## Install

Download binary releases from the [MonyaCode releases](https://github.com/monyacode/monyacode/releases) page.

For development and platform-specific instructions, see the
[documentation](./docs/index.md).

## Build from source

Install the Rust toolchain specified in `rust-toolchain.toml`, then run:

```bash
cargo build --release
```

The default workspace member builds the MonyaCode application. Platform bundle
scripts are available in `script/`.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and
[CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) for contribution guidelines.

## License

MonyaCode modifications are licensed under the GNU GPLv3 or later. This
project also includes source code and dependencies under their original
licenses. See [LICENSE-GPL](./LICENSE-GPL), [LICENSE-APACHE](./LICENSE-APACHE),
and the dependency notices for details.
