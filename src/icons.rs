use image::{Rgba, RgbaImage, DynamicImage};
use png::{BitDepth, ColorType, Encoder, PixelDimensions, Unit};
use std::sync::OnceLock;

use crate::models::ServiceStatus;
use crate::constants::{COLOR_RUNNING, COLOR_STOPPED, STATUS_DOT_SIZE, STATUS_DOT_OFFSET};

pub const BASE_ICON_BYTES: &[u8] =
    include_bytes!("../assets/llama-48.png");        // keep the @2x suffix

/// 1 inch / 0.0254 m × 144 dpi  ≈ 5 669 px per metre
const RETINA_PPM: u32 = 5_669;

/// Cached pre-encoded PNG data for maximum performance
struct IconCache {
    running_png: Vec<u8>,
    stopped_png: Vec<u8>,
    unknown_png: Vec<u8>,
}

static ICON_CACHE: OnceLock<IconCache> = OnceLock::new();

/// Initialize the icon cache (called once at startup)
fn init_icon_cache() -> IconCache {
    // Load and decode the base icon once
    let base_icon = image::load_from_memory(BASE_ICON_BYTES)
        .expect("Failed to load base icon");
    let base_rgba = base_icon.to_rgba8();
    
    // Pre-compute and encode all status variants
    let running_png = create_and_encode_status_icon(&base_rgba, COLOR_RUNNING)
        .expect("Failed to encode running icon");
    let stopped_png = create_and_encode_status_icon(&base_rgba, COLOR_STOPPED)
        .expect("Failed to encode stopped icon");
    let unknown_png = create_and_encode_status_icon(&base_rgba, (128, 128, 128))
        .expect("Failed to encode unknown icon");
    
    IconCache {
        running_png,
        stopped_png,
        unknown_png,
    }
}

/// Create a status icon with dot and encode directly to PNG
fn create_and_encode_status_icon(base: &RgbaImage, color: (u8, u8, u8)) -> crate::Result<Vec<u8>> {
    let mut icon = base.clone();
    draw_status_dot(&mut icon, color);
    encode_rgba_to_png(&icon)
}

/// Ultra-fast status icon generation - returns pre-encoded PNG data
pub fn get_status_icon_png(status: ServiceStatus) -> crate::Result<bitbar::attr::Image> {
    let cache = ICON_CACHE.get_or_init(init_icon_cache);
    
    let png_data = match status {
        ServiceStatus::Running => &cache.running_png,
        ServiceStatus::Stopped => &cache.stopped_png,
        ServiceStatus::Unknown => &cache.unknown_png,
    };
    
    Ok(bitbar::attr::Image::from(png_data.clone()))
}

/// Convert chart image to menu image (for charts only)
pub fn chart_to_menu_image(chart: DynamicImage) -> crate::Result<bitbar::attr::Image> {
    let buffer = encode_rgba_to_png(&chart.to_rgba8())?;
    Ok(bitbar::attr::Image::from(buffer))
}

/// Encode RGBA image to PNG with retina metadata
fn encode_rgba_to_png(rgba: &RgbaImage) -> crate::Result<Vec<u8>> {
    let (w, h) = rgba.dimensions();
    let mut buffer = Vec::new();
    
    {
        let mut encoder = Encoder::new(&mut buffer, w, h);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        
        // Tag as 2× (≈ 144 dpi) so AppKit won't upscale
        encoder.set_pixel_dims(Some(PixelDimensions {
            xppu: RETINA_PPM,
            yppu: RETINA_PPM,
            unit: Unit::Meter,
        }));
        
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba)?;
    }
    
    Ok(buffer)
}

/// Draw the dot only inside its bounding box (≈ 5× faster than naive approach)
fn draw_status_dot(icon: &mut RgbaImage, color: (u8, u8, u8)) {
    let (w, h) = icon.dimensions();
    let r = (STATUS_DOT_SIZE / 2) as i32;
    let cx = w as i32 - STATUS_DOT_OFFSET as i32 - r;
    let cy = h as i32 - STATUS_DOT_OFFSET as i32 - r;
    let r_sq = r * r;
    let px = Rgba([color.0, color.1, color.2, 255]);

    // Only iterate over the bounding box of the circle
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
