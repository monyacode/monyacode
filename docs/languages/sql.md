# SQL

Syntax highlighting and formatting support for SQL is built into the editor.

- [Tree sitter grammar](https://github.com/DerekStride/tree-sitter-sql)

### Formatting

SQL auto-formatting can be added to the editor using external tools like
[`sql-formatter`](https://github.com/sql-formatter-org/sql-formatter).

1. Install `sql-formatter`:

```sh
npm install -g sql-formatter
```

2. Ensure `sql-formatter` is available in your path and check the version:

```sh
which sql-formatter
sql-formatter --version
```

3. Configure MonyaCode to automatically format SQL with `sql-formatter`:

```json
  "languages": {
    "SQL": {
      "formatter": {
        "external": {
          "command": "sql-formatter",
          "arguments": ["--language", "mysql"]
        }
      }
    }
  },
```

Substitute your preferred [SQL Dialect] for `mysql` above (`duckdb`, `hive`,
`mariadb`, `postgresql`, `redshift`, `snowflake`, `sqlite`, `spark`, etc).

You can add this to MonyaCode project settings (`.monyacode/settings.json`) or via your
MonyaCode user settings (`~/.config/monyacode/settings.json`).

### Advanced Formatting

Sql-formatter also allows more precise control by providing
[sql-formatter configuration options](https://github.com/sql-formatter-org/sql-formatter#configuration-options).
To provide these, create a `.sql-formatter.json` file in your project:

```json
{
  "language": "postgresql",
  "tabWidth": 2,
  "keywordCase": "upper",
  "linesBetweenQueries": 2,
}
```

When using a `.sql-formatter.json` file you can use a more simplified set of
MonyaCode settings since the language need not be specified inline:

```json
  "languages": {
    "SQL": {
      "formatter": {
        "external": {
          "command": "sql-formatter"
        }
      }
    }
  },
```
