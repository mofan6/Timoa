# Windows font rasterization adaptation

This directory preserves Slint 1.18.1's software renderer and its upstream
copyright and licenses. The local changes are limited to Windows glyph coverage
generation in `fonts/directwrite.rs`, `fonts/vectorfont.rs`, `fonts.rs`, and the
Windows dependencies in `Cargo.toml`.

- Shape and match fonts through the existing Slint/fontique path. Rasterize the
  same font blob, collection index and glyph ID with DirectWrite.
- Share immutable font bytes rather than copying an entire font for each face.
  Cache at most 16 native faces per thread and use the existing 1 MiB weighted
  glyph cache. Real Regular and Bold faces remain separate.
- Use grayscale glyph coverage on newer Windows. On Windows 7, dynamically fall
  back to the base factory and convert RGB coverage to neutral alpha. There is
  no hard import of a newer DirectWrite factory function.
- Fall back to the original Swash path when native creation/rasterization fails,
  or when a font contains unsupported variable/color tables.
- Bound font streams and raster allocation sizes. Unregister each custom loader
  when its cached face is dropped. Unsafe COM interoperation is permitted only
  in the native adapter module; the crate-wide lint remains in place.

The native adapter follows the existing `vendor/femtovg` adaptation; this copy
feeds Slint's software glyph coverage maps rather than a GPU atlas. These local
changes use the same Slint license alternatives as the surrounding renderer.

Upstream: https://github.com/slint-ui/slint/tree/v1.18.1/internal/renderers/software
See `LICENSES/` and the source SPDX notices for the complete upstream terms.
