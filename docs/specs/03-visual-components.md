# Phase 3: Visual Components Specification

## Overview

This phase implements the visual elements of the plugin: the status icon with colored dot overlay and sparkline charts for metrics visualization.

## Goals

- Create composite status icon (base icon + colored dot)
- Implement sparkline chart generation for metrics
- Integrate with bitbar crate's image handling
- Ensure visual consistency across light/dark themes

## Implementation

### 3.1 Icon System Architecture

Update src/icons.rs with full implementation:

```rust
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
pub fn icon_to_menu_image(icon: DynamicImage) -> crate::Result<bitbar::Image> {
    // The bitbar crate handles PNG encoding and base64 conversion
    icon.try_into()
        .map_err(|e| format!("Failed to convert icon: {:?}", e).into())
}
```

### 3.2 Sparkline Chart Implementation

Create comprehensive chart rendering in src/charts.rs:

```rust
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
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
```

### 3.3 Integration with Menu

Update src/menu.rs to use visual components:

```rust
use bitbar::{Menu, MenuItem};
use crate::{PluginState, icons, charts};
use crate::models::ServiceStatus;

pub fn build_menu(state: &PluginState) -> Menu {
    let mut items = vec![];
    
    // Generate and add title with status icon
    if let Ok(status_icon) = generate_title_item(state.current_status) {
        items.push(status_icon);
    }
    
    items.push(MenuItem::Sep);
    
    // Add service controls
    items.extend(generate_control_items(state.current_status));
    
    items.push(MenuItem::Sep);
    
    // Add metrics if service is running
    if state.current_status == ServiceStatus::Running {
        items.extend(generate_metrics_items(&state.metrics_history));
    } else {
        items.push(MenuItem::new("Service is not running").color("#888888"));
    }
    
    Menu(items)
}

fn generate_title_item(status: ServiceStatus) -> crate::Result<MenuItem> {
    // Generate icon with status dot
    let icon = icons::generate_status_icon(status)?;
    let menu_image = icons::icon_to_menu_image(icon)?;
    
    // Create title item with just the icon (no text)
    Ok(MenuItem::new("").image(menu_image))
}

fn generate_control_items(status: ServiceStatus) -> Vec<MenuItem> {
    let mut items = vec![];
    
    match status {
        ServiceStatus::Running => {
            items.push(
                MenuItem::new("🔴 Stop Llama-Swap")
                    .command(bitbar::Command::restart("do_stop"))
            );
        }
        ServiceStatus::Stopped | ServiceStatus::Unknown => {
            items.push(
                MenuItem::new("🟢 Start Llama-Swap")
                    .command(bitbar::Command::restart("do_start"))
            );
        }
    }
    
    items.push(
        MenuItem::new("⟲ Restart Llama-Swap")
            .command(bitbar::Command::restart("do_restart"))
    );
    
    items
}

fn generate_metrics_items(history: &crate::models::MetricsHistory) -> Vec<MenuItem> {
    let mut items = vec![];
    
    // Section header
    items.push(MenuItem::new("Performance Metrics").color("#888888"));
    
    // TPS with sparkline
    if let Some(&latest_tps) = history.tps.back() {
        if let Ok(sparkline) = charts::generate_tps_sparkline(&history.tps) {
            if let Ok(chart_image) = sparkline.try_into() {
                items.push(
                    MenuItem::new(format!("TPS: {:.1}", latest_tps))
                        .image(chart_image)
                );
            }
        }
    }
    
    // Memory with sparkline
    if let Some(&latest_mem) = history.memory_mb.back() {
        if let Ok(sparkline) = charts::generate_memory_sparkline(&history.memory_mb) {
            if let Ok(chart_image) = sparkline.try_into() {
                items.push(
                    MenuItem::new(format!("Memory: {:.1} MB", latest_mem))
                        .image(chart_image)
                );
            }
        }
    }
    
    // Cache hit rate with sparkline
    if let Some(&latest_cache) = history.cache_hit_rate.back() {
        if let Ok(sparkline) = charts::generate_cache_sparkline(&history.cache_hit_rate) {
            if let Ok(chart_image) = sparkline.try_into() {
                items.push(
                    MenuItem::new(format!("Cache Hit Rate: {:.1}%", latest_cache))
                        .image(chart_image)
                );
            }
        }
    }
    
    items
}

pub fn build_error_menu(message: &str) -> Result<Menu, std::fmt::Error> {
    Ok(Menu(vec![
        MenuItem::new("⚠️ Error"),
        MenuItem::Sep,
        MenuItem::new(message).color("#ff0000"),
    ]))
}
```

### 3.4 Testing Visual Components

Create a test module in src/charts.rs:

```rust
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
```

## Asset Requirements

### Base Icon Specifications

Create `assets/llama-icon.png` with:
- Dimensions: 20x20 pixels (40x40 for Retina)
- Format: PNG with transparency
- Content: Simple, recognizable design
- Leave space in bottom-right for status dot

Example icon creation with ImageMagick:
```bash
# Create a simple placeholder icon
convert -size 20x20 xc:none \
  -fill "#4A90E2" \
  -draw "circle 10,10 10,2" \
  -fill "#FFFFFF" \
  -font Arial -pointsize 12 \
  -gravity center -annotate +0+0 "L" \
  assets/llama-icon.png
```

## Visual Design Guidelines

### Color Choices
- **Running (Green)**: #00C853 - High contrast, visible in both themes
- **Stopped (Red)**: #D50000 - Clear stop indication
- **Charts**: Bright colors that work on dark backgrounds
  - TPS: Spring Green (#00FF7F)
  - Memory: Deep Sky Blue (#00BFFF)
  - Cache: Orange (#FFA500)

### Chart Design
- Transparent background for theme adaptability
- 1-pixel lines for clarity at small size
- Dots on data points only if ≤20 points
- 5% padding to prevent edge clipping

## Performance Considerations

- Icon generation is cached (base icon loaded once)
- Charts are generated on-demand each refresh
- Image operations are fast (~1ms per chart)
- Base64 encoding handled by bitbar crate

## Next Steps

With visual components complete, proceed to [Phase 4: Service Integration](04-service-integration.md) to implement service monitoring and control.