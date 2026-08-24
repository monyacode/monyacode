# Globs

[Glob patterns](<https://en.wikipedia.org/wiki/Glob_(programming)>) is the
formal name for Unix shell-style path matching wildcards like `*.md` or
`docs/**/*.md` supported by sh, bash, zsh, etc. A glob is similar but distinct
from a
[regex (regular expression)](https://en.wikipedia.org/wiki/Regular_expression).
In MonyaCode these are commonly used when matching filenames.

## Glob Flavor

MonyaCode uses two different rust crates for matching glob patterns:

- [ignore crate](https://docs.rs/ignore/latest/ignore/) for matching glob
  patterns stored in `.gitignore` files
- [glob crate](https://docs.rs/glob/latest/glob/) for matching file paths in
  MonyaCode

While simple expressions are portable across environments (e.g. running
`ls *.py` or `*.tmp` in a gitignore) there is significant divergence in the
support for and syntax of more advanced features varies (character classes,
exclusions, `**`, etc) across implementations. The rest of this document will be
describing globs as supported in MonyaCode via the `glob` crate implementation. See
[References](#references) below for documentation links for glob pattern syntax
for `.gitignore`, shells and other programming languages.

The `glob` crate is implemented entirely in rust and does not rely on the `glob`
/ `fnmatch` interfaces provided by libc. This means that globs in MonyaCode should
behave similarly across platforms.

## Introduction

A glob "pattern" is used to match a file name or complete file path. For
example, when using "Search all files" {#kb project_search::ToggleFocus} you can
click the funnel shaped Toggle Filters" button or {#kb
project_search::ToggleFilters} and it will show additional search fields for
"Include" and "Exclude" which support specifying glob patterns for matching file
paths and file names.

When creating a glob pattern you can use one or multiple special characters:

| Special Character | Meaning                                                           |
| ----------------- | ----------------------------------------------------------------- |
| `?`               | Matches any single character                                      |
| `*`               | Matches any (possibly empty) sequence of characters               |
| `**`              | Matches the current directory and arbitrary subdirectories        |
| `[abc]`           | Matches any one character in the brackets                         |
| `[a-z]`           | Matches any of a range of characters (ordered by Unicode)         |
| `[!...]`          | The negation of `[...]` (matches a character not in the brackets) |

Notes:

1. Shell-style brace-expansions like `{a,b,c}` are not supported.
2. To match a literal `-` character inside brackets it must come first `[-abc]`
   or last `[abc-]`.
3. To match the literal `[` character use `[[]` or put it as the first character
   in the group `[[abc]`.
4. To match the literal `]` character use `[]]` or put it as the last character
   in the group `[abc]]`.

## Examples

### Matching file extensions

If you wanted to only search Markdown files add `*.md` to the "Include" search
field.

### Case insensitive matching

Globs in MonyaCode are case-sensitive, so `*.c` will not match `main.C` (even on
case-insensitive filesystems like HFS+/APFS on macOS). Instead use brackets to
match characters. So instead of `*.c` use `*.[cC]`.

### Matching directories

If you wanted to search the
[monyacode repository](https://github.com/monyacode/monyacode) for examples of
[Configuring Language Servers](./configuring-languages#configuring-language-servers.md)
(under `"lsp"` in MonyaCode `settings.jsonc`) you could search for `"lsp"` and in the
"Include" filter specify `docs/**/*.md`. This would only match files whose path
was under the `docs` directory or any nested subdirectories `**/` of that folder
with a filename that ends in `.md`.

If instead you wanted to restrict yourself only to MonyaCode Language-Specific
Documentation pages you could define a narrower pattern of:
`docs/languages/*.md` this would match `docs/languages/rust.md` and
`docs/languages/cpp.md` but not `docs/configuring-languages.md`.

### Implicit Wildcards

When using the "Include" / "Exclude" filters on a Project Search each glob is
wrapped in implicit wildcards. For example to exclude any files with license in
the path or filename from your search just type `license` in the exclude box.
Behind the scenes MonyaCode transforms `license` to `**license**`. This means that
files named `license.*`, `*.license` or inside a `license` subdirectory will all
be filtered out. This enables users to easily filter for `*.ts` without having
to remember to type `**/*.ts` every time.

Alternatively, if in your MonyaCode settings you wanted a
[`file_types`](./configuring-monyacode.md#file-types) override which only applied to
a certain directory you must explicitly include the wildcard globs. For example,
if you had a directory of template files with the `html` extension that you
wanted to recognize as Jinja2 template you could use the following:

```json
{
  "file_types": {
    "C++": ["[cC]"],
    "Jinja2": ["**/templates/*.html"],
  },
}
```

## References

While globs in MonyaCode are implemented as described above, when writing code using
globs in other languages, please reference your platform's glob documentation:

- [macOS fnmatch](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/fnmatch.3.html)
  (BSD C Standard Library)
- [Linux fnmatch](https://www.gnu.org/software/libc/manual/html_node/Wildcard-Matching.html)
  (GNU C Standard Library)
- [POSIX fnmatch](https://pubs.opengroup.org/onlinepubs/9699919799/functions/fnmatch.html)
  (POSIX Specification)
- [node-glob](https://github.com/isaacs/node-glob) (Node.js `glob` package)
- [Python glob](https://docs.python.org/3/library/glob.html) (Python Standard
  Library)
- [Golang glob](https://pkg.go.dev/path/filepath#Match) (Go Standard Library)
- [gitignore patterns](https://git-scm.com/docs/gitignore) (Gitignore Pattern
  Format)
- [PowerShell: About Wildcards](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_wildcards)
  (Wildcards in PowerShell)
