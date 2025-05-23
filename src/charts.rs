use image::{DynamicImage, Rgba, RgbaImage};
use crate::constants::*;
use std::collections::VecDeque;

/// Generate a sparkline chart from data points
pub fn generate_sparkline(
    data: &VecDeque<f64>,
    color: (u8, u8, u8),
    width: u32,
    height: u32,
) -> crate::Result<DynamicImage> {
    // Create transparent image
    let mut img = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    
    if data.is_empty() {
        // Return empty transparent image
        return Ok(DynamicImage::ImageRgba8(img));
    }
    
    // Calculate scaling factors
    let data_vec: Vec<f64> = data.iter().cloned().collect();
    let (min_val, max_val) = calculate_bounds(&data_vec);
    let value_range = max_val - min_val;
    
    // Handle edge case of flat line
    let scale = if value_range > 0.0 {
        (height - 1) as f64 / value_range
    } else {
        0.0
    };
    
    // Calculate x spacing
    let x_step = if data.len() > 1 {
        width as f64 / (data.len() - 1) as f64
    } else {
        0.0
    };
    
    // Draw the sparkline
    draw_line_chart(&mut img, &data_vec, min_val, scale, x_step, color);
    
    Ok(DynamicImage::ImageRgba8(img))
}

/// Calculate min and max with some padding
fn calculate_bounds(data: &[f64]) -> (f64, f64) {
    let min = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    // Add 5% padding to avoid line touching edges
    let padding = (max - min) * 0.05;
    (min - padding, max + padding)
}

/// Draw the line chart on the image
fn draw_line_chart(
    img: &mut RgbaImage,
    data: &[f64],
    min_val: f64,
    scale: f64,
    x_step: f64,
    color: (u8, u8, u8),
) {
    let height = img.height();
    
    // Convert data points to pixel coordinates
    let points: Vec<(u32, u32)> = data
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let x = (i as f64 * x_step) as u32;
            let y = height - 1 - ((value - min_val) * scale) as u32;
            (x.min(img.width() - 1), y.min(height - 1))
        })
        .collect();
    
    // Draw lines between consecutive points
    for window in points.windows(2) {
        draw_line(img, window[0], window[1], color);
    }
    
    // Optionally draw dots at data points for clarity
    if data.len() <= 20 {
        for &(x, y) in &points {
            draw_dot(img, x, y, color);
        }
    }
}

/// Draw a line between two points (basic Bresenham's algorithm)
fn draw_line(
    img: &mut RgbaImage,
    (x0, y0): (u32, u32),
    (x1, y1): (u32, u32),
    color: (u8, u8, u8),
) {
    let dx = (x1 as i32 - x0 as i32).abs();
    let dy = (y1 as i32 - y0 as i32).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    
    let mut x = x0 as i32;
    let mut y = y0 as i32;
    
    loop {
        // Draw pixel with full opacity
        if x >= 0 && y >= 0 && x < img.width() as i32 && y < img.height() as i32 {
            img.put_pixel(x as u32, y as u32, Rgba([color.0, color.1, color.2, 255]));
        }
        
        if x == x1 as i32 && y == y1 as i32 {
            break;
        }
        
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draw a small dot (for data points)
fn draw_dot(img: &mut RgbaImage, cx: u32, cy: u32, color: (u8, u8, u8)) {
    let radius = 1;
    let (width, height) = img.dimensions();
    
    for y in cy.saturating_sub(radius)..=(cy + radius).min(height - 1) {
        for x in cx.saturating_sub(radius)..=(cx + radius).min(width - 1) {
            let dx = x as i32 - cx as i32;
            let dy = y as i32 - cy as i32;
            if dx * dx + dy * dy <= radius as i32 * radius as i32 {
                img.put_pixel(x, y, Rgba([color.0, color.1, color.2, 255]));
            }
        }
    }
}

/// Helper to generate sparklines for specific metrics
pub fn generate_tps_sparkline(history: &VecDeque<f64>) -> crate::Result<DynamicImage> {
    generate_sparkline(history, COLOR_TPS_LINE, CHART_WIDTH, CHART_HEIGHT)
}

pub fn generate_memory_sparkline(history: &VecDeque<f64>) -> crate::Result<DynamicImage> {
    generate_sparkline(history, COLOR_MEM_LINE, CHART_WIDTH, CHART_HEIGHT)
}

pub fn generate_cache_sparkline(history: &VecDeque<f64>) -> crate::Result<DynamicImage> {
    generate_sparkline(history, COLOR_CACHE_LINE, CHART_WIDTH, CHART_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sparkline_generation() {
        let mut data = VecDeque::new();
        for i in 0..10 {
            data.push_back(i as f64);
        }
        
        let result = generate_sparkline(&data, (255, 0, 0), 60, 15);
        assert!(result.is_ok());
        
        let img = result.unwrap();
        assert_eq!(img.width(), 60);
        assert_eq!(img.height(), 15);
    }
    
    #[test]
    fn test_empty_data() {
        let data = VecDeque::new();
        let result = generate_sparkline(&data, (255, 0, 0), 60, 15);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_single_point() {
        let mut data = VecDeque::new();
        data.push_back(42.0);
        
        let result = generate_sparkline(&data, (255, 0, 0), 60, 15);
        assert!(result.is_ok());
    }
}