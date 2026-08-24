# Developing MonyaCode

See the platform-specific instructions for building MonyaCode from source:

- [macOS](./development/macos.md)
- [Linux](./development/linux.md)
- [Windows](./development/windows.md)

## System Requirements

Rust is notoriously memory hungry, and MonyaCode is a huge Rust project. The minimum
requirement for compiling the project seems to be somewhere around 16GB of RAM,
and at least 8GB available for compilation.

Linking is the part of the process which takes the most RAM, and alternative
linkers may offer better performance and lower RAM usage. Check the platform
specific instructions for more details on compiling with an alternative linker
like `wild` or `mold`.

If you have a lot of CPU cores, Rust will try to use all of them and may require
even more RAM as a consequence. You can limit the level of concurrency by
setting the `CARGO_BUILD_JOBS` environment variable. For example, if you have 16
CPU cores available, Cargo will run up to 16 jobs in parallel. By setting
`CARGO_BUILD_JOBS=8` you can reduce RAM usage by only running up to 8 jobs in
parallel.

To build with less than 12GB of RAM, try setting these environment variables:

```sh
MAKEOPTS="-j2 -l2"
CARGO_BUILD_JOBS=1
```

It's also possible that using an alternative linker like `wild` or `mold` may
use less memory.

## Performance Measurements

MonyaCode includes a frame time measurement system that can be used to profile how
long it takes to render each frame. This is particularly useful when comparing
rendering performance between different versions or when optimizing frame
rendering code.

### Using MONYACODE_MEASUREMENTS

To enable performance measurements, set the `MONYACODE_MEASUREMENTS` environment
variable:

```sh
export MONYACODE_MEASUREMENTS=1
```

When enabled, MonyaCode will print frame rendering timing information to stderr,
showing how long each frame takes to render.

### Performance Comparison Workflow

Here's a typical workflow for comparing frame rendering performance between
different versions:

1. **Enable measurements:**

   ```sh
   export MONYACODE_MEASUREMENTS=1
   ```

2. **Test the first version:**

   - Checkout the commit you want to measure
   - Run MonyaCode in release mode and use it for 5-10 seconds:
     `cargo run --release &> version-a`

3. **Test the second version:**

   - Checkout another commit you want to compare
   - Run MonyaCode in release mode and use it for 5-10 seconds:
     `cargo run --release &> version-b`

4. **Generate comparison:**

   ```sh
   script/histogram version-a version-b
   ```

The `script/histogram` tool can accept as many measurement files as you like and
will generate a histogram visualization comparing the frame rendering
performance data between the provided versions.

### Using `util_macros::perf`

For benchmarking unit tests, annotate them with the `#[perf]` attribute from the
`util_macros` crate. Then run `cargo perf-test -p $CRATE` to benchmark them. See
the rustdoc documentation on `crates/util_macros` and `tooling/perf` for
in-depth examples and explanations.
