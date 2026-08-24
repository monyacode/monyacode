# Bash

Bash language support is provided natively by MonyaCode.

- Tree-sitter:
  [tree-sitter/tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash)
- Language Server:
  [bash-lsp/bash-language-server](https://github.com/bash-lsp/bash-language-server)

## Configuration

### `shellcheck`

`bash-language-server` supports `shellcheck` and will make use of it
automatically, if installed. This must be done manually by the user.

```sh
brew install shellcheck             # macOS (HomeBrew)
apt install shellcheck              # Ubuntu / Debian
pacman -S shellcheck                # Arch Linux
dnf install shellcheck              # Fedora
yum install shellcheck              # CentOS/RHEL
zypper install shellcheck           # openSUSE
choco install shellcheck            # Windows (Chocolatey)
```

And verify it is available from your path:

```sh
which shellcheck
shellcheck --version
```

If you wish to customize the warnings/errors reported you just need to create a
`.shellcheckrc` file. You can do this in the root of your project or in your
home directory (`~/.shellcheckrc`). See:
[shellcheck documentation](https://github.com/koalaman/shellcheck/wiki/Ignore#ignoring-one-or-more-types-of-errors-forever)
for more.

### `smfmt`

Similarly, `bash-language-server` will make automatic use of `shfmt` for
formatting, if it is available in `$PATH`. Install it with:

```sh
brew install shfmt          # MacOS (HomeBrew)
apt install shfmt           # Ubuntu / Debian
pacman -S shfmt             # Arch Linux
choco install shfmt         # Windows (chocolatey)
```

To control automatic formatting you can use `format_on_save` in your MonyaCode
settings or manually invoke `editor: format document` from the command palette.

### See also:

- [MonyaCode Docs: Language Support: Shell Scripts](./sh.md)
