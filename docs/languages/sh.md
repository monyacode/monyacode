# Shell Scripts

Shell Scripts (bash, zsh, dash, sh) are supported natively by MonyaCode.

- Tree-sitter:
  [tree-sitter/tree-sitter-bash](https://github.com/tree-sitter/tree-sitter-bash)

## Settings

You can configure various settings for Shell Scripts in your MonyaCode User Settings
(`~/.config/monyacode/settings.json`) or MonyaCode Project Settings
(`.monyacode/settings.json`):

```json
  "languages": {
    "Shell Script": {
      "tab_size": 2,
      "hard_tabs": false
    }
  }
```

### Formatting

MonyaCode supports auto-formatting Shell Scripts using external tools like
[`shfmt`](https://github.com/mvdan/sh).

1. Install `shfmt`:

```sh
brew install shfmt            # macos (homebrew)
sudo apt-get install shfmt    # debian/ubuntu
dnf install shfmt             # fedora
yum install shfmt             # redhat
pacman -Sy shfmt              # archlinux
choco install shfmt           # windows (chocolatey)
```

2. Ensure `shfmt` is available in your path and check the version:

```sh
which shfmt
shfmt --version
```

3. Configure MonyaCode to automatically format Shell Scripts with `shfmt` on save:

```json
  "languages": {
    "Shell Script": {
      "format_on_save": "on",
      "formatter": {
        "external": {
          "command": "shfmt",
          // Change `--indent 2` to match your preferred tab_size
          "arguments": ["--filename", "{buffer_path}", "--indent", "2"]
        }
      }
    }
  }
```

## See also:

- [Language Support for Bash](./bash.md)
- [Language Support for Fish](./fish.md)
