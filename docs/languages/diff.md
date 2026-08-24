# Diff

Diff support is available natively in MonyaCode.

- Tree-sitter:
  [zed-industries/the-mikedavis/tree-sitter-diff](https://github.com/the-mikedavis/tree-sitter-diff)

## Configuration

MonyaCode will not attempt to format diff files and has
[`remove_trailing_whitespace_on_save`](../configuring-monyacode.md#remove-trailing-whitespace-on-save)
and
[`ensure-final-newline-on-save`](../configuring-monyacode.md#ensure-final-newline-on-save)
set to false.

MonyaCode will automatically recognize files with `patch` and `diff` extensions as
Diff files. To recognize other extensions, add them to `file_types` in your MonyaCode
`settings.jsonc`:

```json
  "file_types": {
    "Diff": ["dif"]
  },
```
