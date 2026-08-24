# Building MonyaCode for FreeBSD

Note, FreeBSD is not currently a supported platform, and so this is a
work-in-progress.

## Repository

Clone the [MonyaCode repository](https://github.com/monyacode/monyacode).

## Dependencies

- Install the necessary system packages and rustup:

  ```sh
  script/freebsd
  ```

  If preferred, you can inspect
  [`script/freebsd`](https://github.com/zed-industries/zed/blob/main/script/freebsd)
  and perform the steps manually.

## Building from source

Once the dependencies are installed, you can build MonyaCode using
[Cargo](https://doc.rust-lang.org/cargo/).

For a debug build of the editor:

```sh
cargo run
```

And to run the tests:

```sh
cargo test --workspace
```

In release mode, the primary user interface is the `cli` crate. You can run it
in development with:

```sh
cargo run -p cli
```

## Troubleshooting

### Cargo errors claiming that a dependency is using unstable features

Try `cargo clean` and `cargo build`.
