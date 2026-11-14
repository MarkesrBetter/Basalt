use image::{GenericImageView, RgbaImage};
use std::path::Path;

use crate::config::SkinModel;

const DEFAULT_SKIN_PNG: &[u8] = include_bytes!("../assets/default_skin.png");

pub fn default_skin_png_bytes() -> &'static [u8] {
    DEFAULT_SKIN_PNG
}

/// Bundled default 64x64 Slim player skin decoded from assets/default_skin.png.
pub fn generate_default_skin() -> RgbaImage {
    image::load_from_memory(DEFAULT_SKIN_PNG)
        .expect("embedded assets/default_skin.png must decode as PNG")
        .to_rgba8()
}

fn region_has_visible_pixel(img: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
    for y in y0..y1 {
        for x in x0..x1 {
            if img.get_pixel(x, y)[3] > 0 {
                return true;
            }
        }
    }
    false
}

/// Inspects 64x64 UV arm columns to detect 3-pixel Slim (Alex) vs 4-pixel Classic (Steve).
pub fn detect_skin_is_slim(img: &RgbaImage) -> bool {
    if img.width() != 64 || img.height() != 64 {
        return false;
    }
    let has_right_arm = region_has_visible_pixel(img, 40, 20, 54, 32);
    if !has_right_arm {
        return false;
    }
    let right_arm_tail_empty = !region_has_visible_pixel(img, 54, 20, 56, 32);
    let left_arm_tail_empty = !region_has_visible_pixel(img, 46, 52, 48, 64);
    let right_sleeve_tail_empty = !region_has_visible_pixel(img, 54, 36, 56, 48);
    right_arm_tail_empty && left_arm_tail_empty && right_sleeve_tail_empty
}

/// Resolves whether a skin uses the 3-pixel Slim model given explicit setting or Auto mode.
pub fn resolve_skin_is_slim(model: SkinModel, img: &RgbaImage) -> bool {
    match model {
        SkinModel::Slim => true,
        SkinModel::Classic => false,
        SkinModel::Auto => detect_skin_is_slim(img),
    }
}

/// Loads a user-supplied local PNG skin file.
/// Supports standard 64x64 skins and legacy 64x32 skins (auto-converted to 64x64 layout).
pub fn load_skin_from_file(path: &Path) -> Result<RgbaImage, String> {
    let dyn_img = image::open(path).map_err(|e| format!("Skin dosyası açılamadı: {e}"))?;
    let (w, h) = dyn_img.dimensions();
    let rgba = dyn_img.to_rgba8();

    if w == 64 && h == 64 {
        Ok(rgba)
    } else if w == 64 && h == 32 {
        Ok(convert_legacy_64x32_to_64x64(&rgba))
    } else if w >= 64 && w == h && w.is_power_of_two() {
        Ok(image::imageops::resize(
            &rgba,
            64,
            64,
            image::imageops::FilterType::Nearest,
        ))
    } else {
        Err(format!(
            "Desteklenmeyen skin boyutu ({w}x{h}). 64x64 veya 64x32 PNG kullanın."
        ))
    }
}

fn convert_legacy_64x32_to_64x64(src: &RgbaImage) -> RgbaImage {
    let mut dst = RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
    for y in 0..32 {
        for x in 0..64 {
            dst.put_pixel(x, y, *src.get_pixel(x, y));
        }
    }
    copy_region(src, &mut dst, 0, 16, 16, 48, 16, 16);
    copy_region(src, &mut dst, 40, 16, 32, 48, 16, 16);
    dst
}

fn copy_region(
    src: &RgbaImage,
    dst: &mut RgbaImage,
    sx: u32,
    sy: u32,
    dx: u32,
    dy: u32,
    w: u32,
    h: u32,
) {
    for y in 0..h {
        for x in 0..w {
            dst.put_pixel(dx + x, dy + y, *src.get_pixel(sx + x, sy + y));
        }
    }
}
