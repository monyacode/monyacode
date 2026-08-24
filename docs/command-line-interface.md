# Command-line Interface

MonyaCode has a CLI, on Linux this should come with the distribution's MonyaCode package
(binary name can vary from distribution to distribution, `monyacode` will be used
later for brevity). For macOS, the CLI comes in the same package with the editor
binary, and could be installed into the system with the `cli: install` MonyaCode
command which will create a symlink to the `/usr/local/bin/monyacode`. It can also be
built from source out of the `cli` crate in this repository.

Use `monyacode --help` to see the full list of capabilities. General highlights:

- Opening another empty MonyaCode window: `monyacode`

- Opening a file or directory in MonyaCode: `monyacode /path/to/entry` (use `-n` to open
  in the new window)

- Reading from stdin: `ps axf | monyacode -`

- Starting MonyaCode with logs in the terminal: `monyacode --foreground`

- Uninstalling MonyaCode and all its related files: `monyacode --uninstall`
