# Zlog

Use the `MONYACODE_LOG` environment variable to control logging output for MonyaCode
applications and libraries. The variable accepts a comma-separated list of
directives that specify logging levels for different modules (crates). The
general format is for instance:

```
MONYACODE_LOG=info,project=debug
```

- Levels can be one of: `off`/`none`, `error`, `warn`, `info`, `debug`, or
  `trace`.
- You don't need to specify the global level, default is `trace` in the crate
  and `info` set by `RUST_LOG` in MonyaCode.
