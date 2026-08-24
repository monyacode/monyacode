# Cli

## Testing

You can test your changes to the `cli` crate by first building the main monyacode
binary:

```
cargo build -p monyacode
```

And then building and running the `cli` crate with the following parameters:

```
 cargo run -p cli -- --monyacode ./target/debug/monyacode.exe
```
