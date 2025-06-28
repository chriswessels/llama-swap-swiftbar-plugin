use image::{DynamicImage, Rgba, RgbaImage};
use crate::constants::{COLOR_TPS_LINE, COLOR_MEM_LINE, COLOR_PROMPT_LINE, COLOR_KV_CACHE_LINE, CHART_WIDTH, CHART_HEIGHT};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub enum MetricType {
    Tps,
    Memory,
    Prompt,
    KvCache,
}

impl MetricType {
    fn color(self) -> (u8, u8, u8) {
        match self {
            Self::Tps => COLOR_TPS_LINE,
            Self::Memory => COLOR_MEM_LINE,
            Self::Prompt => COLOR_PROMPT_LINE,
            Self::KvCache => COLOR_KV_CACHE_LINE,
        }
    }
}

/// Generate a sparkline chart with semantic colors and smart bounds
pub fn generate_sparkline(
    data: &VecDeque<f64>,
    metric_type: MetricType,
) -> crate::Result<DynamicImage> {
    generate_sparkline_with_size(data, metric_type, CHART_WIDTH, CHART_HEIGHT)
}

/// Generate a sparkline chart with custom dimensions
pub fn generate_sparkline_with_size(
    data: &VecDeque<f64>,
    metric_type: MetricType,
    width: u32,
    height: u32,
) -> crate::Result<DynamicImage> {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    
    if data.is_empty() {
        return Ok(DynamicImage::ImageRgba8(img));
    }
    
    let data_vec: Vec<f64> = data.iter().copied().collect();
    let (min_val, max_val) = calculate_bounds(&data_vec);
    let scale = if max_val > min_val {
        f64::from(height - 1) / (max_val - min_val)
    } else {
        0.0
    };
    
    let x_step = if data.len() > 1 {
        f64::from(width) / (data.len() - 1) as f64
    } else {
        0.0
    };
    
    draw_line_chart(&mut img, &data_vec, min_val, scale, x_step, metric_type.color());
    
    Ok(DynamicImage::ImageRgba8(img))
}


/// Smart bounds calculation that centers data and maximizes use of chart space
fn calculate_bounds(data: &[f64]) -> (f64, f64) {
    if data.is_empty() {
        return (0.0, 1.0);
    }
    
    let min = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let range = max - min;
    
    if range.abs() < f64::EPSILON {
        let value = min;
        let padding = value.abs().max(1.0) * 0.5;
        (value - padding, value + padding)
    } else {
        let center = f64::midpoint(min, max);
        let half_range = range / 2.0;
        let padding = half_range * 0.05;
        let expanded_half_range = half_range + padding;
        (center - expanded_half_range, center + expanded_half_range)
    }
}


/// Draw line chart with optional dots for sparse data
fn draw_line_chart(
    img: &mut RgbaImage,
    data: &[f64],
    min_val: f64,
    scale: f64,
    x_step: f64,
    color: (u8, u8, u8),
) {
    let height = img.height();
    
    let points: Vec<(u32, u32)> = data
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let x = (i as f64 * x_step) as u32;
            let y = height - 1 - ((value - min_val) * scale) as u32;
            (x.min(img.width() - 1), y.min(height - 1))
        })
        .collect();
    
    for window in points.windows(2) {
        draw_line(img, window[0], window[1], color);
    }
}

/// Draw a line between two points using Bresenham's algorithm
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
    let rgba = Rgba([color.0, color.1, color.2, 255]);
    
    loop {
        if x >= 0 && y >= 0 && x < img.width() as i32 && y < img.height() as i32 {
            img.put_pixel(x as u32, y as u32, rgba);
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


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sparkline_generation() {
        let mut data = VecDeque::new();
        for i in 0..10 {
            data.push_back(f64::from(i));
        }
        
        let result = generate_sparkline(&data, MetricType::Tps);
        assert!(result.is_ok());
        
        let img = result.unwrap();
        assert_eq!(img.width(), CHART_WIDTH);
        assert_eq!(img.height(), CHART_HEIGHT);
    }
    
    #[test]
    fn test_empty_data() {
        let data = VecDeque::new();
        let result = generate_sparkline(&data, MetricType::Memory);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_single_point() {
        let mut data = VecDeque::new();
        data.push_back(42.0);
        
        let result = generate_sparkline(&data, MetricType::Prompt);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_custom_size() {
        let mut data = VecDeque::new();
        data.push_back(1.0);
        data.push_back(2.0);
        
        let result = generate_sparkline_with_size(&data, MetricType::KvCache, 100, 20);
        assert!(result.is_ok());
        
        let img = result.unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 20);
    }
}