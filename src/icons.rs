use image::{ImageBuffer, Rgb, RgbImage};
use crate::models::ServiceStatus;
use crate::constants::*;
use crate::Result;

// Embed the base icon at compile time
pub const BASE_ICON_BYTES: &[u8] = include_bytes!("../assets/llama-icon.png");

pub fn generate_status_icon(status: ServiceStatus) -> Result<String> {
    // TODO: Implement icon generation with status overlay
    todo!("Implement icon generation")
}