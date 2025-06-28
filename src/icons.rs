use image::{Rgba, RgbaImage, DynamicImage};
use png::{BitDepth, ColorType, Encoder, PixelDimensions, Unit};
use std::sync::OnceLock;

use crate::state_machines::program::ProgramStates;
use crate::constants::{STATUS_DOT_SIZE, STATUS_DOT_OFFSET, 
    COLOR_PROCESSING_QUEUE, COLOR_MODEL_READY, COLOR_MODEL_LOADING, 
    COLOR_SERVICE_NO_MODEL, COLOR_AGENT_STARTING, COLOR_AGENT_NOT_LOADED};

use base64::{engine::general_purpose::STANDARD as B64, Engine};

pub const DARK_BASE_ICON_BYTES: &[u8] =
    include_bytes!("../assets/llama-48-white.png");

pub const LIGHT_BASE_ICON_BYTES: &[u8] =
    include_bytes!("../assets/llama-48.png");

/// 1 inch / 0.0254 m × 144 dpi  ≈ 5 669 px per metre
const RETINA_PPM: u32 = 5_669;

/// Cached icon images for maximum performance
struct IconCache {
    processing_queue: bitbar::attr::Image,
    model_ready: bitbar::attr::Image,
    model_loading: bitbar::attr::Image,
    service_no_model: bitbar::attr::Image,
    agent_starting: bitbar::attr::Image,
    agent_not_loaded: bitbar::attr::Image,
}

static ICON_CACHE: OnceLock<IconCache> = OnceLock::new();

/// Initialize the icon cache (called once at startup)
fn init_icon_cache() -> IconCache {
    // Load and decode the base icons once
    let base_icon_dark = image::load_from_memory(DARK_BASE_ICON_BYTES)
        .expect("Failed to load dark base icon");
    let base_rgba_dark = base_icon_dark.to_rgba8();
    
    let base_icon_light = image::load_from_memory(LIGHT_BASE_ICON_BYTES)
        .expect("Failed to load light base icon");
    let base_rgba_light = base_icon_light.to_rgba8();
    
    // Create themed images for each program state
    let processing_queue = create_themed_status_icon(&base_rgba_light, &base_rgba_dark, COLOR_PROCESSING_QUEUE)
        .expect("Failed to create processing queue icon");
    let model_ready = create_themed_status_icon(&base_rgba_light, &base_rgba_dark, COLOR_MODEL_READY)
        .expect("Failed to create model ready icon");
    let model_loading = create_themed_status_icon(&base_rgba_light, &base_rgba_dark, COLOR_MODEL_LOADING)
        .expect("Failed to create model loading icon");
    let service_no_model = create_themed_status_icon(&base_rgba_light, &base_rgba_dark, COLOR_SERVICE_NO_MODEL)
        .expect("Failed to create service no model icon");
    let agent_starting = create_themed_status_icon(&base_rgba_light, &base_rgba_dark, COLOR_AGENT_STARTING)
        .expect("Failed to create agent starting icon");
    let agent_not_loaded = create_themed_status_icon(&base_rgba_light, &base_rgba_dark, COLOR_AGENT_NOT_LOADED)
        .expect("Failed to create agent not loaded icon");
    
    IconCache {
        processing_queue,
        model_ready,
        model_loading,
        service_no_model,
        agent_starting,
        agent_not_loaded,
    }
}

/// Create a themed status icon (light,dark format) with status dot
fn create_themed_status_icon(light_base: &RgbaImage, dark_base: &RgbaImage, color: (u8, u8, u8)) -> crate::Result<bitbar::attr::Image> {
    // Create light version
    let mut light_icon = light_base.clone();
    draw_status_dot(&mut light_icon, color);
    let light_png = encode_rgba_to_png(&light_icon)?;
    
    // Create dark version
    let mut dark_icon = dark_base.clone();
    draw_status_dot(&mut dark_icon, color);
    let dark_png = encode_rgba_to_png(&dark_icon)?;
    
    // ── turn the raw bytes into text ─────────────────────────────
    let light_b64 = B64.encode(&light_png);  // String
    let dark_b64  = B64.encode(&dark_png);   // String

    // one comma → SwiftBar shows first in Light Mode, second in Dark Mode
    let themed_image_data = format!("{light_b64},{dark_b64}");
    Ok(bitbar::attr::Image::from(themed_image_data))
}

/// Get cached program state icon image
pub fn get_program_state_icon(state: ProgramStates) -> &'static bitbar::attr::Image {
    let cache = ICON_CACHE.get_or_init(init_icon_cache);
    
    match state {
        ProgramStates::ModelProcessingQueue => &cache.processing_queue,
        ProgramStates::ModelReady => &cache.model_ready,
        ProgramStates::ModelLoading => &cache.model_loading,
        ProgramStates::ServiceLoadedNoModel => &cache.service_no_model,
        ProgramStates::AgentStarting => &cache.agent_starting,
        ProgramStates::AgentNotLoaded => &cache.agent_not_loaded,
    }
}

/// Convert chart image to menu image (for charts only)
pub fn chart_to_menu_image(chart: DynamicImage) -> crate::Result<bitbar::attr::Image> {
    let buffer = encode_rgba_to_png(&chart.to_rgba8())?;
    let b64_data = B64.encode(&buffer);
    Ok(bitbar::attr::Image::from(b64_data))
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
