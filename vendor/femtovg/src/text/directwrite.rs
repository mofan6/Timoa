//! Native Windows coverage maps for the existing grayscale glyph atlas.
//! Font bytes and collection indices come from the shaped face, so glyph IDs
//! never get rematched against a potentially different system font.
use std::{ffi::c_void, mem::ManuallyDrop, sync::Arc};
use windows::core::{implement, Error, Interface, Result, HRESULT};
use windows::Win32::Graphics::DirectWrite::*;

pub(super) type SharedFontBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;

pub(super) struct GlyphImage {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<u8>,
}

fn invalid_argument() -> Error {
    Error::from_hresult(HRESULT(0x80070057_u32 as i32))
}

#[implement(IDWriteFontFileLoader)]
struct MemoryLoader {
    bytes: SharedFontBytes,
}

impl IDWriteFontFileLoader_Impl for MemoryLoader_Impl {
    fn CreateStreamFromKey(&self, key: *const c_void, key_size: u32) -> Result<IDWriteFontFileStream> {
        if key.is_null() || key_size != std::mem::size_of::<u64>() as u32 {
            return Err(invalid_argument());
        }
        Ok(MemoryStream { bytes: self.bytes.clone() }.into())
    }
}

#[implement(IDWriteFontFileStream)]
struct MemoryStream {
    bytes: SharedFontBytes,
}

impl IDWriteFontFileStream_Impl for MemoryStream_Impl {
    fn ReadFileFragment(&self, start: *mut *mut c_void, offset: u64, size: u64, context: *mut *mut c_void) -> Result<()> {
        let offset = usize::try_from(offset).map_err(|_| invalid_argument())?;
        let size = usize::try_from(size).map_err(|_| invalid_argument())?;
        let end = offset.checked_add(size).ok_or_else(invalid_argument)?;
        let bytes = self.bytes.as_ref().as_ref();
        if start.is_null() || context.is_null() || end > bytes.len() {
            return Err(invalid_argument());
        }
        // The immutable Arc allocation remains alive for the COM stream's
        // lifetime, including fragments retained by DirectWrite internally.
        unsafe {
            start.write(bytes.as_ptr().add(offset).cast_mut().cast());
            context.write(std::ptr::null_mut());
        }
        Ok(())
    }

    fn ReleaseFileFragment(&self, _context: *mut c_void) {}
    fn GetFileSize(&self) -> Result<u64> { Ok(self.bytes.as_ref().as_ref().len() as u64) }
    fn GetLastWriteTime(&self) -> Result<u64> { Ok(0) }
}

pub(super) struct FontFace {
    face: Option<IDWriteFontFace>,
    factory: IDWriteFactory,
    modern: Option<IDWriteFactory2>,
    loader: IDWriteFontFileLoader,
}

impl Drop for FontFace {
    fn drop(&mut self) {
        // Release the face/file before unregistering its callback provider.
        self.face.take();
        unsafe { let _ = self.factory.UnregisterFontFileLoader(&self.loader); }
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}

fn supported_face(bytes: &[u8], index: u32) -> Option<()> {
    // Color and variable fonts keep the upstream rasterizer. This preserves
    // emoji colors, variation coordinates, and the existing timer digits.
    let offset = if bytes.get(..4)? == b"ttcf" {
        if index >= read_u32(bytes, 8)? { return None; }
        read_u32(bytes, 12_usize.checked_add((index as usize).checked_mul(4)?)?)? as usize
    } else {
        if index != 0 { return None; }
        0
    };
    let count = read_u16(bytes, offset.checked_add(4)?)? as usize;
    if count > 4096 { return None; }
    for entry in 0..count {
        let tag_offset = offset.checked_add(12)?.checked_add(entry.checked_mul(16)?)?;
        let tag = bytes.get(tag_offset..tag_offset.checked_add(4)?)?;
        if matches!(tag, b"fvar" | b"COLR" | b"CBDT" | b"sbix" | b"SVG ") { return None; }
    }
    Some(())
}

impl FontFace {
    pub(super) fn new(bytes: SharedFontBytes, index: u32, embolden: bool) -> Option<Self> {
        supported_face(bytes.as_ref().as_ref(), index)?;
        if bytes.as_ref().as_ref().len() > 64 * 1024 * 1024 { return None; }
        unsafe {
            // DWriteCreateFactory is available from Windows 7. Newer factory
            // interfaces are queried dynamically rather than hard imported.
            let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).ok()?;
            // Share the original immutable font blob with shaping instead of
            // allocating a second full copy for each native rasterized face.
            let loader: IDWriteFontFileLoader = MemoryLoader { bytes }.into();
            factory.RegisterFontFileLoader(&loader).ok()?;
            let mut result = Self {
                face: None, modern: factory.cast().ok(), factory, loader,
            };
            let key = 0_u64;
            let file = result.factory.CreateCustomFontFileReference(
                (&key as *const u64).cast(), std::mem::size_of_val(&key) as u32, &result.loader,
            ).ok()?;
            let mut supported = Default::default();
            let mut file_type = DWRITE_FONT_FILE_TYPE::default();
            let mut face_type = DWRITE_FONT_FACE_TYPE::default();
            let mut face_count = 0;
            file.Analyze(&mut supported, &mut file_type, Some(&mut face_type), &mut face_count).ok()?;
            if !supported.as_bool() || index >= face_count { return None; }
            result.face = Some(result.factory.CreateFontFace(
                face_type, &[Some(file)], index,
                if embolden { DWRITE_FONT_SIMULATIONS_BOLD } else { DWRITE_FONT_SIMULATIONS_NONE },
            ).ok()?);
            Some(result)
        }
    }

    pub(super) fn rasterize(&self, size: f32, glyph: u16, subpixel_x: f32) -> Option<GlyphImage> {
        if !size.is_finite() || !(1.0..=1024.0).contains(&size) || !subpixel_x.is_finite() { return None; }
        let face = self.face.as_ref()?;
        let advance = 0.0_f32;
        let offset = DWRITE_GLYPH_OFFSET::default();
        let mut run = DWRITE_GLYPH_RUN {
            fontFace: ManuallyDrop::new(Some(face.clone())), fontEmSize: size,
            glyphCount: 1, glyphIndices: &glyph, glyphAdvances: &advance,
            glyphOffsets: &offset, isSideways: false.into(), bidiLevel: 0,
        };
        unsafe {
            let modern = self.modern.as_ref().and_then(|factory| factory.CreateGlyphRunAnalysis(
                &run, None, DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC,
                DWRITE_MEASURING_MODE_NATURAL, DWRITE_GRID_FIT_MODE_ENABLED,
                DWRITE_TEXT_ANTIALIAS_MODE_GRAYSCALE, subpixel_x, 0.0,
            ).ok());
            let grayscale = modern.is_some();
            let analysis = modern.or_else(|| self.factory.CreateGlyphRunAnalysis(
                &run, 1.0, None, DWRITE_RENDERING_MODE_NATURAL_SYMMETRIC,
                DWRITE_MEASURING_MODE_NATURAL, subpixel_x, 0.0,
            ).ok());
            ManuallyDrop::drop(&mut run.fontFace);
            let analysis = analysis?;
            let texture = if grayscale { DWRITE_TEXTURE_ALIASED_1x1 } else { DWRITE_TEXTURE_CLEARTYPE_3x1 };
            let bounds = analysis.GetAlphaTextureBounds(texture).ok()?;
            let width = u32::try_from(bounds.right.checked_sub(bounds.left)?).ok()?;
            let height = u32::try_from(bounds.bottom.checked_sub(bounds.top)?).ok()?;
            if width == 0 || height == 0 || width > 2048 || height > 2048 { return None; }
            let pixels = (width as usize).checked_mul(height as usize)?;
            let channels = if grayscale { 1 } else { 3 };
            let mut coverage = vec![0; pixels.checked_mul(channels)?];
            analysis.CreateAlphaTexture(texture, &bounds, &mut coverage).ok()?;
            if !grayscale {
                // Windows 7 exposes RGB coverage. Fold it to a neutral mask
                // for the existing alpha atlas, including translucent layers.
                coverage = coverage.chunks_exact(3).map(|rgb| {
                    ((u16::from(rgb[0]) + u16::from(rgb[1]) + u16::from(rgb[2]) + 1) / 3) as u8
                }).collect();
            }
            Some(GlyphImage { left: bounds.left, top: bounds.top.checked_neg()?, width, height, coverage })
        }
    }
}
