# Sarasa UI SC common-character subsets

The Chinese interface uses the UI SC variant of Sarasa Gothic 1.0.42.
Regular (400) and Bold (700) are subsetted with fontTools. Retained glyph
outlines, original metrics and hinting are preserved; headings select the real
Bold face without a synthetic stroke overlay. English keeps Segoe UI.

Coverage includes all current application/catalog text in three languages,
Latin, common punctuation, fullwidth forms and a modest editable reserve from
`tools/font-common-characters.txt`. Other user text uses platform fallback.
Regenerate subsets from the full upstream masters:

```
python tools/subset-ui-fonts.py --master-dir PATH_TO_FULL_SARASA_UI_SC_FONTS
```

When new CJK UI characters are missing, build.rs invokes this generator if
`TIMO_FONT_PYTHON` and `TIMO_FONT_MASTER_DIR` are set (and fontTools is
available to that Python). The development preview build configures them.
Otherwise the build gives a clear instruction instead of shipping missing
Chinese UI glyphs. The coverage file is a build input, not a runtime resource.

The subset's internal family is `Timoa Sarasa UI SC`, distinct from an
installed full font. `manifest.json` records exact source and output hashes,
sizes, coverage and tool version. Full masters remain in the development
backup and are available from the linked official upstream release.

Copyright (c) 2015-2025 Renzhi Li, with upstream contributions from Inter,
Adobe and Google. Full copyright notices and the SIL Open Font License 1.1
are preserved in `../../licenses/Sarasa-Gothic-OFL.txt` and the executable's
third-party license viewer.

Both simplified and traditional Chinese UI text use this family. Timer digits
retain their separate existing font. This is a modified/subset font, not an
upstream unmodified release.
