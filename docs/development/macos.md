# Building MonyaCode for macOS

## Repository

Clone down the [MonyaCode repository](https://github.com/monyacode/monyacode).

```zsh
# Your apple ID (email)
export APPLE_ID=""
# App-specific password (create in account.apple.com)
export APPLE_PASSWORD_MONYACODE=""
# Apple Team ID (find it in XCode)
export APPLE_TEAM_ID=""
# Apple signing key: security find-identity -p codesigning
export APPLE_SIGNING_KEY=""
# Build, sign and notarise the app bundle
./script/bundle-mac
```

## Dependencies

- Install [rustup](https://www.rust-lang.org/tools/install)

- Install [Xcode](https://apps.apple.com/us/app/xcode/id497799835?mt=12) from
  the macOS App Store, or from the
  [Apple Developer](https://developer.apple.com/download/all/) website. Note
  this requires a developer account.

> Ensure you launch Xcode after installing, and install the macOS components,
> which is the default option.

- Install
  [Xcode command line tools](https://developer.apple.com/xcode/resources/)

  ```zsh
  xcode-select --install
  ```

- Ensure that the Xcode command line tools are using your newly installed copy
  of Xcode:

  ```zsh
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  sudo xcodebuild -license accept
  ```

- Install `cmake` (required by
  [a dependency](https://docs.rs/wasmtime-c-api-impl/latest/wasmtime_c_api/))

  ```zsh
  brew install cmake
  ```

## Building MonyaCode from Source

Once you have the dependencies installed, you can build MonyaCode using
[Cargo](https://doc.rust-lang.org/cargo/).

For a debug build:

```zsh
cargo run
```

For a release build:

```zsh
cargo run --release
```

And to run the tests:

```zsh
cargo test --workspace
```

## Troubleshooting

### Error compiling metal shaders

```zsh
error: failed to run custom build command for gpui v0.1.0 (/Users/path/to/monyacode)`**
xcrun: error: unable to find utility "metal", not a developer tool or in PATH
```

Try `sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer`

If you're on macOS 26, try `xcodebuild -downloadComponent MetalToolchain`

### Cargo errors claiming that a dependency is using unstable features

Try `cargo clean` and `cargo build`.

### Error: 'dispatch/dispatch.h' file not found

If you encounter an error similar to:

```zsh
src/platform/mac/dispatch.h:1:10: fatal error: 'dispatch/dispatch.h' file not found

Caused by:
  process didn't exit successfully

  --- stdout
  cargo:rustc-link-lib=framework=System
  cargo:rerun-if-changed=src/platform/mac/dispatch.h
  cargo:rerun-if-env-changed=TARGET
  cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS_aarch64-apple-darwin
  cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin
  cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS
```

This file is part of Xcode. Ensure you have installed the Xcode command line
tools and set the correct path:

```zsh
xcode-select --install
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

Additionally, set the `BINDGEN_EXTRA_CLANG_ARGS` environment variable:

```zsh
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$(xcrun --show-sdk-path)"
```

Then clean and rebuild the project:

```zsh
cargo clean
cargo run
```

### Tests failing due to `Too many open files (os error 24)`

This error seems to be caused by OS resource constraints. Installing and running
tests with `cargo-nextest` should resolve the issue.

- `cargo install cargo-nextest --locked`
- `cargo nextest run --workspace --no-fail-fast`

## Tips & Tricks

### Avoiding continual rebuilds

If you are finding that MonyaCode is continually rebuilding root crates, it may be
because you are pointing your development MonyaCode at the codebase itself.

This causes problems because `cargo run` exports a bunch of environment
variables which are picked up by the `rust-analyzer` that runs in the
development build of MonyaCode. These environment variables are in turn passed to
`cargo check`, which invalidates the build cache of some of the crates we depend
on.

You can easily avoid running the built binary on the checked-out MonyaCode codebase
using `cargo run ~/path/to/other/project` to ensure that you don't hit this.

### Speeding up verification

If you are building MonyaCode a lot, you may find that macOS continually verifies new
builds which can add a few seconds to your iteration cycles.

To fix this, you can:

- Run `sudo spctl developer-mode enable-terminal` to enable the Developer Tools
  panel in System Settings.
- In System Settings, search for "Developer Tools" and add your terminal (e.g.
  iTerm or Ghostty) to the list under "Allow applications to use developer
  tools"
- Restart your terminal.

Thanks to the nextest developers for publishing
[this](https://nexte.st/docs/installation/macos/#gatekeeper).
