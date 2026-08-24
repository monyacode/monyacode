# Building MonyaCode for Linux

## Repository

Clone the [MonyaCode repository](https://github.com/monyacode/monyacode).

## Dependencies

- Install [rustup](https://www.rust-lang.org/tools/install)

- Install the necessary system libraries:

  ```sh
  script/linux
  ```

  If you prefer to install the system libraries manually, you can find the list
  of required packages in the `script/linux` file.

### Linkers {#linker}

On Linux, Rust's default linker is
[LLVM's `lld`](https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/). Alternative
linkers, especially [Wild](https://github.com/davidlattimore/wild) and
[Mold](https://github.com/rui314/mold) can significantly improve clean and
incremental build time.

For local development Wild is recommended because it's 5-20% faster than Mold.

These linkers can be installed with `script/install-mold` and
`script/install-wild`.

To use Wild as your default, add these lines to your `~/.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=--ld-path=wild"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=--ld-path=wild"]
```

To use Mold as your default:

```toml
[target.'cfg(target_os = "linux")']
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

## Building from source

Once the dependencies are installed, you can build using
[Cargo](https://doc.rust-lang.org/cargo/).

For a debug build of the editor:

```sh
cargo run
```

To run the tests:

```sh
cargo test --workspace
```

In release mode, the primary user interface is the `cli` crate. You can run it
in development with:

```sh
cargo run -p cli
```

## Installing a development build

You can install a local build on your machine with:

```sh
./script/install.sh --build
```

This will build monyacode and the cli in release mode and make them available at
`~/.local/bin/monyacode`, installing .desktop files to `~/.local/share`.

> **_Note_**: If you encounter linker errors similar to the following:
>
> ```bash
> error: linking with `cc` failed: exit status: 1 ...
> = note: /usr/bin/ld: /tmp/rustcISMaod/libaws_lc_sys-79f08eb6d32e546e.rlib(f8e4fd781484bd36-bcm.o): in function `aws_lc_0_25_0_handle_cpu_env':
>           /aws-lc/crypto/fipsmodule/cpucap/cpu_intel.c:(.text.aws_lc_0_25_0_handle_cpu_env+0x63): undefined reference to `__isoc23_sscanf'
>           /usr/bin/ld: /tmp/rustcISMaod/libaws_lc_sys-79f08eb6d32e546e.rlib(f8e4fd781484bd36-bcm.o): in function `pkey_rsa_ctrl_str':
>           /aws-lc/crypto/fipsmodule/evp/p_rsa.c:741:(.text.pkey_rsa_ctrl_str+0x20d): undefined reference to `__isoc23_strtol'
>           /usr/bin/ld: /aws-lc/crypto/fipsmodule/evp/p_rsa.c:752:(.text.pkey_rsa_ctrl_str+0x258): undefined reference to `__isoc23_strtol'
>           collect2: error: ld returned 1 exit status
>   = note: some `extern` functions couldn't be found; some native libraries may need to be installed or have their path specified
>   = note: use the `-l` flag to specify native libraries to link
>   = note: use the `cargo:rustc-link-lib` directive to specify the native libraries to link with Cargo (see https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-lib)
> error: could not compile `remote_server` (bin "remote_server") due to 1 previous error
> ```
>
> **Cause**: this is caused by known bugs in aws-lc-rs(doesn't support GCC >=
> 14):
> [FIPS fails to build with GCC >= 14](https://github.com/aws/aws-lc-rs/issues/569)
> &
> [GCC-14 - build failure for FIPS module](https://github.com/aws/aws-lc/issues/2010)
>
> You can refer to
> [linux: Linker error for remote_server when using script/install-linux](https://github.com/zed-industries/zed/issues/24880)
> for more information.
>
> **Workarounds**: Set the remote server target to `x86_64-unknown-linux-gnu`
> like so
> `export REMOTE_SERVER_TARGET=x86_64-unknown-linux-gnu; script/install-linux`

## Wayland & X11

MonyaCode supports both X11 and Wayland. By default, we pick whichever we can find at
runtime. If you're on Wayland and want to run in X11 mode, use the environment
variable `WAYLAND_DISPLAY=''`.

## Packaging MonyaCode

MonyaCode has two main binaries:

- You will need to build `crates/cli` and make its binary available in `$PATH`
  with the name `monyacode`.
- You will need to build `crates/monyacode` and put it at
  `$PATH/to/cli/../../libexec/monyacode-editor`. For example, if you are going to put
  the cli at `~/.local/bin/monyacode` put monyacode at `~/.local/libexec/monyacode-editor`. As
  some linux distributions (notably Arch) discourage the use of `libexec`, you
  can also put this binary at `$PATH/to/cli/../../lib/monyacode/monyacode-editor` (e.g.
  `~/.local/lib/monyacode/monyacode-editor`) instead.
- If you are going to provide a `.desktop` file you can find a template in
  `crates/monyacode/resources/monyacode.desktop.in`, and use `envsubst` to populate it
  with the values required. This file should also be renamed to
  `$APP_ID.desktop` so that the file
  [follows the FreeDesktop standards](https://github.com/zed-industries/zed/issues/12707#issuecomment-2168742761).
  You should also make this desktop file executable (`chmod 755`).
- You will need to ensure that the necessary libraries are installed. You can
  get the current list by
  [inspecting the built binary](https://github.com/zed-industries/zed/blob/935cf542aebf55122ce6ed1c91d0fe8711970c82/script/bundle-linux#L65-L67)
  on your system.
- For an example of a complete build script, see
  [script/bundle-linux](https://github.com/zed-industries/zed/blob/935cf542aebf55122ce6ed1c91d0fe8711970c82/script/bundle-linux).
- Set the environment variable `MONYACODE_NO_BUNDLED_UNINSTALL` to something like
  "Use flatpak to uninstall" so that the CLI uninstall command doesn't try to
  run the uninstallation script.

### Other things to note

Zed comes with some (in our opinion) pretty severe caveats:

- It automatically downloads and installs binaries without explicit user
  prompting in the same way as rustup/rbenv/pyenv, etc. They say that they
  "understand this is contentious", and yet they insist on doing it.
  [See here](https://github.com/zed-industries/zed/issues/12589).
- They download and install extension binaries from
  [zed-industries/extensions](https://github.com/zed-industries/extensions)
  without explicit permission or any way to inspect what is being installed.
  These extensions may in turn install further software without explicit
  permission or control. For more details,
  [see here](https://github.com/zed-industries/zed/issues/12358).
- It does not play nice with sandboxes,
  [see here](https://github.com/zed-industries/zed/pull/12006#issuecomment-2130421220)

Our (MonyaCode's) intention is to resolve all of these problems.

## Distro Packages

The MonyaCode project includes scripts to build binary packages for various targets,
including `.deb`, `.rpm` and `AppImage`. The driver for all these is the
`scripts/bundle-linux` script which takes the following flags (among others):

- `--deb` (requires [`cargo-deb`](https://github.com/kornelski/cargo-deb))
- `--rpm` (requires
  [`cargo-generate-rpm`](https://github.com/cat-in-136/cargo-generate-rpm))
- `--appimage` (requires
  [`pkg2appimage`](https://github.com/AppImageCommunity/pkg2appimage))

Adding any of these will build the corresponding package. Note that the
`AppImage` is created from the `.deb.` package so while you can only provide the
`--appimage` flag it will automatically build a `.deb` as well.

Note that each of these package builds requires an additional dependency as
noted above.

## Flatpak

> MonyaCode's current Flatpak integration exits the sandbox on startup. Workflows
> that rely on Flatpak's sandboxing may not work as expected.

To build & install the Flatpak package locally follow the steps below:

1. Install Flatpak for your distribution as outlined
   [here](https://flathub.org/setup).
2. Run `script/linux` to install Linux dependencies.
3. Run `script/flatpak/deps` script to install the required dependencies.
4. Run `script/flatpak/bundle-flatpak`.
5. Now the package has been installed and has a bundle available at
   `target/release/{app-id}.flatpak`.

## Memory profiling

[`heaptrack`](https://github.com/KDE/heaptrack) is quite useful for diagnosing
memory leaks. To install it:

```sh
$ sudo apt install heaptrack heaptrack-gui
$ cargo install cargo-heaptrack
```

Then, to build and run MonyaCode with the profiler attached:

```sh
$ cargo heaptrack -b monyacode
```

When this editor instance is exited, terminal output will include a command to
run `heaptrack_interpret` to convert the `*.raw.zst` profile to a `*.zst` file
which can be passed to `heaptrack_gui` for viewing.

## Perf recording

This section describes how to generate a flamegraph with resolved symbols from a
running editor instance. This can be used to debug when the editor is using a
lot of CPU. It's not useful for debugging hangs or memory usage.

### During the incident

- Find the PID (process ID) using:
  `ps -eo size,pid,comm | grep monyacode | sort | head -n 1 | cut -d ' ' -f 2` Or
  find the pid of the command monyacode-editor with the most ram usage in something
  like htop/btop/top.

- Install perf: On Ubuntu (derivatives) run `sudo apt install linux-tools`.

- Perf Record: run `sudo perf record -p <pid you just found>`, wait a few
  seconds to gather data then press Ctrl+C. You should now have a perf.data file

- Make the output file user owned: run `sudo chown $USER:$USER perf.data`

- Get build info: Run monyacode again and type `monyacode: about` in the command pallet to
  get the exact commit.

### Later

- Build MonyaCode with symbols:
  - Check out the commit found previously and modify `Cargo.toml`.
  - Apply the following diff, then make a release build.

    ```diff
    [profile.release]
    -debug = "limited"
    +debug = "full"
    ```

- Add the symbols to perf database:
  `pref buildid-cache -v -a <path to release monyacode binary>`

- Resolve the symbols from the db:
  `perf inject -i perf.data -o perf_with_symbols.data`

- Install flamegraph: `cargo install cargo-flamegraph`

- Render the flamegraph: `flamegraph --perfdata perf_with_symbols.data`

## Troubleshooting

### Cargo errors claiming that a dependency is using unstable features

Try `cargo clean` and `cargo build`.
