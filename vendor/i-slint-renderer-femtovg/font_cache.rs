// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0

// cspell:ignore Noto fontconfig

use femtovg::TextContext;
use i_slint_common::sharedfontique::HashedBlob;
use i_slint_core::textlayout::sharedparley::parley;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct FontCache {
    pub(crate) text_context: femtovg::TextContext,
    // Regular and synthesized-bold masks must never share a FontId/atlas key.
    fonts: HashMap<(HashedBlob, u32, bool), femtovg::FontId>,
}

impl Default for FontCache {
    fn default() -> Self {
        let text_context = TextContext::default();
        Self { text_context, fonts: Default::default() }
    }
}

impl FontCache {
    pub fn font(&mut self, font: &parley::FontData, embolden: bool) -> femtovg::FontId {
        let text_context = self.text_context.clone();

        *self.fonts.entry((font.data.clone().into(), font.index, embolden)).or_insert_with(move || {
            text_context.add_shared_font_with_index_and_embolden(font.data.clone(), font.index, embolden).unwrap()
        })
    }
}

thread_local! {
    pub static FONT_CACHE: RefCell<FontCache> = RefCell::new(Default::default())
}
