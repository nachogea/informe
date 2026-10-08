# Brand assets

A folded report with three horizontal bars: a document that can contain data.
The desktop icon adds a second sheet and an indigo tile. The mark uses no
lettering, so it works at small sizes and across surfaces.

- [app-icon.svg](app-icon.svg): editable source for the desktop icon.
- [app-icon.png](app-icon.png): 1024 × 1024 PNG for the README and previews.
- [AppIcon.icns](AppIcon.icns): macOS icon, with 16–1024 pixel representations.
- [mark.svg](mark.svg): flat logo for light backgrounds.

These original assets use the repository's [MIT license](../../LICENSE).
The icon is the first Informe identity draft.

To regenerate PNG and ICNS assets on macOS:

```sh
sh scripts/generate-icons.sh
```

This uses the existing Rust/resvg dependency and macOS `iconutil`. Ordinary app
builds copy the checked-in ICNS; they do not regenerate it. Edit the SVG source,
regenerate, and inspect at 16, 32, 128, and 512 pixels before committing changes.
