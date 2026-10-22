# Local font synthesis adapter

This is Slint 1.18.1's FemtoVG renderer, with its original source notices and
license texts retained. The existing Slint Royalty-free 2.0 license applies.

The upstream glyph-run callback receives Fontique's synthesis request but
ignores it. The local adapter forwards only the `embolden` flag to FemtoVG.
Its font cache now includes that flag in the key. Real bold faces already
selected by the shaper do not get additional emboldening. Shaping, glyph IDs,
metrics, positions, variable-font coordinates and brushes are unchanged.

The paired FemtoVG adapter uses DirectWrite's font-face bold simulation and
creates a single grayscale coverage mask. Unsupported native faces use
Swash's outline embolden before its single mask is rasterized. The former UI
stroke layer has been removed, so separately rasterized fill/stroke glyphs
cannot produce misaligned overlapping contours.

No new Windows API imports are required. Regular and synthesized-bold faces
have distinct FontIds, native font faces and glyph-atlas entries.
