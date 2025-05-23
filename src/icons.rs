use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use crate::constants::*;
use crate::models::ServiceStatus;

// Embed the base icon at compile time
pub const BASE_ICON_BYTES: &[u8] = include_bytes!("../assets/llama-icon.png");

/// Generate status icon with colored dot overlay
pub fn generate_status_icon(status: ServiceStatus) -> crate::Result<DynamicImage> {
    // Load base icon
    let base_icon = image::load_from_memory(BASE_ICON_BYTES)?;
    
    // Convert to RGBA for manipulation
    let mut icon = base_icon.to_rgba8();
    
    // Determine dot color based on status
    let dot_color = match status {
        ServiceStatus::Running => COLOR_RUNNING,
        ServiceStatus::Stopped => COLOR_STOPPED,
        ServiceStatus::Unknown => (128, 128, 128), // Gray
    };
    
    // Draw status dot
    draw_status_dot(&mut icon, dot_color);
    
    Ok(DynamicImage::ImageRgba8(icon))
}

/// Draw a circular status dot on the icon
fn draw_status_dot(icon: &mut RgbaImage, color: (u8, u8, u8)) {
    let (width, height) = icon.dimensions();
    
    // Calculate dot position (bottom-right corner)
    let dot_center_x = width - STATUS_DOT_OFFSET - STATUS_DOT_SIZE / 2;
    let dot_center_y = height - STATUS_DOT_OFFSET - STATUS_DOT_SIZE / 2;
    let radius = STATUS_DOT_SIZE / 2;
    
    // Draw filled circle
    for y in 0..height {
        for x in 0..width {
            let dx = x as i32 - dot_center_x as i32;
            let dy = y as i32 - dot_center_y as i32;
            let distance_sq = dx * dx + dy * dy;
            
            if distance_sq <= (radius * radius) as i32 {
                // Inside circle - set pixel to dot color
                icon.put_pixel(x, y, Rgba([color.0, color.1, color.2, 255]));
            }
        }
    }
}

/// Convert icon to bitbar Image for menu display
pub fn icon_to_menu_image(icon: DynamicImage) -> crate::Result<bitbar::attr::Image> {
    // Convert to PNG bytes
    let mut buffer = Vec::new();
    icon.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageOutputFormat::Png)?;
    
    // Create bitbar Image from PNG bytes
    Ok(bitbar::attr::Image::from(buffer))
}