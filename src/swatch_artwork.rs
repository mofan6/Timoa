//! Size-aware theme artwork. Match physical output dimensions and apply the
//! display scale to logical corner geometry, avoiding bitmap stretching.
use slint::{Color, Image};
use std::{cell::RefCell, collections::VecDeque};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    width_px: u16,
    height_px: u16,
    scale_milli: u16,
    rgb: [u8; 3],
    selected: bool,
    hovered: bool,
    pressed: bool,
}

thread_local! {
    // Resizing and hovering must not accumulate an unbounded set of SVG trees.
    static CACHE: RefCell<VecDeque<(Key, Image)>> = RefCell::new(VecDeque::new());
}

pub fn render(color: Color, width: f32, height: f32, scale: f32, selected: bool, hovered: bool, pressed: bool) -> Image {
    if !width.is_finite() || !height.is_finite() || !scale.is_finite() || width < 16. || height < 16. || scale <= 0. {
        return Image::default();
    }
    let key = Key {
        width_px: width.min(16384.).round() as u16,
        height_px: height.min(2048.).round() as u16,
        scale_milli: (scale.min(8.) * 1000.).round() as u16,
        rgb: [color.red(), color.green(), color.blue()], selected, hovered, pressed,
    };
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.iter().position(|entry| entry.0 == key) {
            let entry = cache.remove(index).unwrap();
            let image = entry.1.clone();
            cache.push_front(entry);
            return image;
        }
        let width = f32::from(key.width_px);
        let height = f32::from(key.height_px);
        let scale = f32::from(key.scale_milli) / 1000.;
        let shade = |factor: f32| {
            let [r, g, b] = key.rgb.map(|channel| (f32::from(channel) * factor).round().clamp(0., 255.) as u8);
            format!("#{r:02x}{g:02x}{b:02x}")
        };
        let bright = shade(if hovered { 1.16 } else { 1.10 });
        let middle = shade(1.);
        let dim = shade(1. / if pressed { 1.18 } else { 1.08 });
        // A constant inset and radius keep face and outline concentric at every
        // width. Neither the corners nor the stroke are stretched with the row.
        let inset = 3.5 * scale;
        let face_width = width - 2. * inset;
        let face_height = height - 2. * inset;
        let radius = (10. * scale).min(face_width / 2.).min(face_height / 2.);
        let edge = 0.75 * scale;
        let outline = if selected {
            format!(r#"<rect x="{edge}" y="{edge}" width="{}" height="{}" rx="{}" fill="none" stroke="url(#paint)" stroke-width="{scale}" opacity="0.8"/>"#,
                width - 2. * edge, height - 2. * edge, radius + inset - edge)
        } else { String::new() };
        // Intrinsic dimensions match the actual control, so the software
        // renderer never widens a narrow, low-resolution intermediate bitmap.
        // Deliberately avoid the old blurred mask around the face.
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><defs><linearGradient id="paint" gradientUnits="userSpaceOnUse" x1="{inset}" y1="{inset}" x2="{}" y2="{}"><stop offset="0" stop-color="{bright}"/><stop offset=".48" stop-color="{middle}"/><stop offset="1" stop-color="{dim}"/></linearGradient></defs>{outline}<rect x="{inset}" y="{inset}" width="{face_width}" height="{face_height}" rx="{radius}" fill="url(#paint)"/></svg>"#,
            width - inset, height - inset);
        let Ok(image) = Image::load_from_svg_data(svg.as_bytes()) else { return Image::default(); };
        cache.push_front((key, image.clone()));
        if cache.len() > 32 { cache.pop_back(); }
        image
    })
}
