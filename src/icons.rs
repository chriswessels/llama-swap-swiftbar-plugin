use image::{DynamicImage, Rgba, RgbaImage};
use png::{BitDepth, ColorType, Encoder, PixelDimensions, Unit};

use crate::models::ServiceStatus;
use crate::constants::{COLOR_RUNNING, COLOR_STOPPED, STATUS_DOT_SIZE, STATUS_DOT_OFFSET};

pub const BASE_ICON_BYTES: &[u8] =
    include_bytes!("../assets/llama-48.png");        // keep the @2x suffix

/// 1 inch / 0.0254 m × 144 dpi  ≈ 5 669 px per metre
const RETINA_PPM: u32 = 5_669;

/// Generate status icon with coloured dot overlay
pub fn generate_status_icon(status: ServiceStatus) -> crate::Result<DynamicImage> {
    let base_icon = image::load_from_memory(BASE_ICON_BYTES)?;
    let mut icon = base_icon.to_rgba8();

    draw_status_dot(&mut icon, get_status_color(status));
    Ok(DynamicImage::ImageRgba8(icon))
}

/// Convert icon to SwiftBar-ready PNG with 2× metadata
pub fn icon_to_menu_image(icon: DynamicImage) -> crate::Result<bitbar::attr::Image> {
    let rgba = icon.to_rgba8();
    let (w, h) = rgba.dimensions();

    let mut buffer = Vec::new();
    {
        let mut encoder = Encoder::new(&mut buffer, w, h);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);

        // Tag the bitmap as 2× (≈ 144 dpi) so AppKit won't upscale it
        encoder.set_pixel_dims(Some(PixelDimensions {
            xppu: RETINA_PPM,
            yppu: RETINA_PPM,
            unit: Unit::Meter,
        }));

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba)?;
    }

    Ok(bitbar::attr::Image::from(buffer))
}

fn get_status_color(status: ServiceStatus) -> (u8, u8, u8) {
    match status {
        ServiceStatus::Running  => COLOR_RUNNING,
        ServiceStatus::Stopped  => COLOR_STOPPED,
        ServiceStatus::Unknown  => (128, 128, 128),
    }
}

/// Draw the dot only inside its bounding box (≈ 5× faster)
fn draw_status_dot(icon: &mut RgbaImage, color: (u8, u8, u8)) {
    let (w, h) = icon.dimensions();
    let r = (STATUS_DOT_SIZE / 2) as i32;
    let cx = w as i32 - STATUS_DOT_OFFSET as i32 - r;
    let cy = h as i32 - STATUS_DOT_OFFSET as i32 - r;
    let r_sq = r * r;
    let px = Rgba([color.0, color.1, color.2, 255]);

    for y in (cy - r).max(0)..=(cy + r).min(h as i32 - 1) {
        for x in (cx - r).max(0)..=(cx + r).min(w as i32 - 1) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r_sq {
                icon.put_pixel(x as u32, y as u32, px);
            }
        }
    }
}
