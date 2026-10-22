# Windows glyph coverage adapter

Upstream: FemtoVG 0.27.0. Original MIT / Apache-2.0 notices and authors are retained.

Timoa changes:

- Windows static outline fonts use DirectWrite coverage maps inside the existing grayscale glyph atlas.
- The exact shaped font bytes and TTC face index are passed through an immutable, bounds-checked COM stream.
- Windows 8.1+ can use the queried Factory2 grayscale/grid-fit interface. Windows 7 uses the base factory and neutralizes its RGB mask for the alpha atlas.
- Unsupported fonts, variable fonts, color fonts, errors and non-Windows platforms retain the upstream Swash route.
- Font layout, advances, glyph placement, canvas composition and application timer behavior are unchanged.
- Requested synthetic bold is applied by DirectWrite to the font face before its single glyph mask is rasterized. Swash's outline embolden is used on the fallback path. No separate stroke pass is needed.
- The paired Slint renderer keys fonts by both their source face and synthesis flag, so regular and synthesized-bold glyphs have distinct atlas entries.

The native face owns its font stream and unregisters its loader after releasing its face. It is cached per FemtoVG Font instance, not by a reusable raw pointer. Font data remains available throughout all DirectWrite callbacks.

This adapter does not enable RGB ClearType blending into transparent layers. Both native routes produce neutral coverage for compatibility with light/dark UI and alpha composition.
