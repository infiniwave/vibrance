use std::sync::Arc;

use gpui::{App, Image, ImageCacheError, ImageFormat, RenderImage, SharedString, Window};

pub mod icon;
pub mod player;
pub mod sidebar;
pub mod sidebar_item;
pub mod track_list;
pub mod track_list_item;

pub fn render_image(
    w: &mut Window,
    a: &mut App,
    bytes: Vec<u8>,
) -> Result<Arc<RenderImage>, ImageCacheError> {
    // detect image format from magic bytes
    let format = if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        ImageFormat::Png
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ImageFormat::Jpeg
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        ImageFormat::Webp
    } else if bytes.starts_with(b"GIF") {
        ImageFormat::Gif
    } else if bytes.starts_with(&[0x42, 0x4D]) {
        ImageFormat::Bmp
    } else {
        // fallback to JPEG
        ImageFormat::Jpeg
    };
    let img = Image::from_bytes(format, bytes);
    Arc::new(img)
        .get_render_image(w, a)
        .ok_or(ImageCacheError::Asset(SharedString::new("")))
}
