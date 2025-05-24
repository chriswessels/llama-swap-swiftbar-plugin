use image::{DynamicImage, Rgba, RgbaImage};
use crate::constants::*;
use crate::models::ServiceStatus;

pub const BASE_ICON_BYTES: &[u8] = include_bytes!("../assets/llama-icon.png");

/// Generate status icon with colored dot overlay
pub fn generate_status_icon(status: ServiceStatus) -> crate::Result<DynamicImage> {
    let base_icon = image::load_from_memory(BASE_ICON_BYTES)?;
    let mut icon = base_icon.to_rgba8();
    
    let dot_color = get_status_color(status);
    draw_status_dot(&mut icon, dot_color);
    
    Ok(DynamicImage::ImageRgba8(icon))
}

/// Convert icon to bitbar Image for menu display
pub fn icon_to_menu_image(icon: DynamicImage) -> crate::Result<bitbar::attr::Image> {
    let mut buffer = Vec::new();
    icon.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageOutputFormat::Png)?;
    Ok(bitbar::attr::Image::from(buffer))
}

fn get_status_color(status: ServiceStatus) -> (u8, u8, u8) {
    match status {
        ServiceStatus::Running => COLOR_RUNNING,
        ServiceStatus::Stopped => COLOR_STOPPED,
        ServiceStatus::Unknown => (128, 128, 128),
    }
}

fn draw_status_dot(icon: &mut RgbaImage, color: (u8, u8, u8)) {
    let (width, height) = icon.dimensions();
    
    let dot_center_x = width - STATUS_DOT_OFFSET - STATUS_DOT_SIZE / 2;
    let dot_center_y = height - STATUS_DOT_OFFSET - STATUS_DOT_SIZE / 2;
    let radius = STATUS_DOT_SIZE / 2;
    let radius_sq = (radius * radius) as i32;
    let rgba = Rgba([color.0, color.1, color.2, 255]);
    
    for y in 0..height {
        for x in 0..width {
            let dx = x as i32 - dot_center_x as i32;
            let dy = y as i32 - dot_center_y as i32;
            
            if dx * dx + dy * dy <= radius_sq {
                icon.put_pixel(x, y, rgba);
            }
        }
    }
}