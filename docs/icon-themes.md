# Icon Themes

MonyaCode comes with a built-in icon theme, with more icon themes available as
extensions.

## Enabling Icon Themes

File and folder icons are disabled by default. You can enable them in
`Settings > Panels > File Icons` and `Settings > Panels > File Icons`.

## Selecting an Icon Theme

See what icon themes are installed and preview them via the Icon Theme Selector,
which you can open from the command palette with `icon theme selector: toggle`.

Navigating through the icon theme list by moving up and down will change the
icon theme in real time and hitting enter will save it to your settings file.

## Installing more Icon Themes

MonyaCode can use icon themes created for the Zed editor. If you find an icon theme
you like, you can install it using the repository URL from the Extensions page.

## Configuring Icon Themes

Your selected icon theme is stored in your settings file. You can open your
settings file from the command palette with {#action monyacode::OpenSettingsFile}
(bound to {#kb monyacode::OpenSettingsFile}).

Just like with themes, MonyaCode allows for configuring different icon themes for
light and dark mode. You can set the mode to `"light"` or `"dark"` to ignore the
current system mode.

```json
{
  "icon_theme": {
    "mode": "system",
    "light": "Light Icon Theme",
    "dark": "Dark Icon Theme",
  },
}
```

## Icon Theme Development

See: [Developing MonyaCode Icon Themes](./extensions/icon-themes.md)
