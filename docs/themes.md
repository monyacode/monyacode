# Themes

MonyaCode comes with a number of built-in themes, with more themes available as
extensions.

## Selecting a Theme

See what themes are installed and preview them via the Theme Selector, which you
can open from the command palette with `theme selector: toggle` (bound to {#kb
theme_selector::Toggle}).

Navigating through the theme list by moving up and down will change the theme in
real time and hitting enter will save it to your settings file.

## Installing more Themes

MonyaCode can use Zed themes so you can install any theme from
[zed-themes.com](https://zed-themes.com), a gallery with visible previews for
many of them.

## Configuring a Theme

Your selected theme is stored in your settings file. You can open your settings
file from the command palette with {#action monyacode::OpenSettingsFile} (bound to
{#kb monyacode::OpenSettingsFile}).

By default, MonyaCode maintains two themes: one for light mode and one for dark mode.
You can set the mode to `"dark"` or `"light"` to ignore the current system mode.

```json
{
  "theme": {
    "mode": "system",
    "light": "One Light",
    "dark": "One Dark",
  },
}
```

## Theme Overrides

To override specific attributes of a theme, use the `theme_overrides` setting.
This setting can be used to configure theme-specific overrides.

For example, add the following to your `settings.json` if you wish to override
the background color of the editor and display comments and doc comments as
italics:

```json
{
  "theme_overrides": {
    "One Dark": {
      "editor.background": "#333",
      "syntax": {
        "comment": {
          "font_style": "italic",
        },
        "comment.doc": {
          "font_style": "italic",
        },
      },
      "accents": [
        "#ff0000",
        "#ff7f00",
        "#ffff00",
        "#00ff00",
        "#0000ff",
        "#8b00ff",
      ],
    },
  },
}
```

To see a comprehensive list of list of captures (like `comment` and
`comment.doc`) see
[Language Extensions: Syntax highlighting](./extensions/languages.md#syntax-highlighting).

To see a list of available theme attributes look at the JSON file for your
theme.

## Local Themes

Store new themes locally by placing them in the `~/.config/monyacode/themes`
directory (macOS and Linux) or `%USERPROFILE%\AppData\Roaming\MonyaCode\themes\`
(Windows).

For example, to create a new theme called `my-cool-theme`, create a file called
`my-cool-theme.json` in that directory. It will be available in the theme
selector the next time MonyaCode loads.

## Theme Development

See: [Developing MonyaCode Themes](./extensions/themes.md)
