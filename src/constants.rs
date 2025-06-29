use std::env;
use std::sync::LazyLock;

// Service configuration
pub const LAUNCH_AGENT_LABEL: &str = "com.user.llama-swap"; // This one stays const as it's rarely changed

// API configuration (configurable via env vars)
pub static API_BASE_URL: LazyLock<String> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1".to_string())
});

pub static API_PORT: LazyLock<u16> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(45786)
});

pub static API_TIMEOUT_SECS: LazyLock<u64> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_API_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
});

// Update timing (configurable via env vars)
pub static STREAMING_MODE: LazyLock<bool> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_STREAMING_MODE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(true)
});

// Chart configuration (configurable via env vars)
pub static CHART_WIDTH: LazyLock<u32> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_CHART_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
});

pub static CHART_HEIGHT: LazyLock<u32> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_CHART_HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
});

pub static HISTORY_SIZE: LazyLock<usize> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_HISTORY_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300) // 5 minutes at 1-second intervals
});

// File paths (configurable via env vars, using home directory expansion)
pub static LOG_FILE_PATH: LazyLock<String> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_LOG_FILE_PATH")
        .unwrap_or_else(|_| "~/Library/Logs/LlamaSwap.log".to_string())
});

pub static CONFIG_FILE_PATH: LazyLock<String> = LazyLock::new(|| {
    env::var("LLAMA_SWAP_CONFIG_FILE_PATH")
        .unwrap_or_else(|_| "~/.llamaswap/config.yaml".to_string())
});

pub const COLOR_TPS_LINE: (u8, u8, u8) = (0, 255, 127); // Spring green - Generation speed
pub const COLOR_PROMPT_LINE: (u8, u8, u8) = (255, 215, 0); // Gold - Prompt speed
pub const COLOR_MEM_LINE: (u8, u8, u8) = (0, 191, 255); // Deep sky blue - Memory
pub const COLOR_QUEUE_LINE: (u8, u8, u8) = (255, 99, 71); // Tomato - Queue size

// Program state color palette (RGB)
pub const COLOR_BLUE: (u8, u8, u8) = (0, 122, 255); // Processing/Active
pub const COLOR_GREEN: (u8, u8, u8) = (52, 199, 89); // Ready/Success
pub const COLOR_YELLOW: (u8, u8, u8) = (255, 255, 0); // Loading/Starting
pub const COLOR_GREY: (u8, u8, u8) = (142, 142, 147); // Idle/No Model
pub const COLOR_RED: (u8, u8, u8) = (255, 59, 48); // Error/Not Loaded

// Semantic color mappings for program states
pub const COLOR_PROCESSING_QUEUE: (u8, u8, u8) = COLOR_BLUE;
pub const COLOR_MODEL_READY: (u8, u8, u8) = COLOR_GREEN;
pub const COLOR_MODEL_LOADING: (u8, u8, u8) = COLOR_YELLOW;
pub const COLOR_SERVICE_NO_MODEL: (u8, u8, u8) = COLOR_GREY;
pub const COLOR_SERVICE_STOPPED: (u8, u8, u8) = COLOR_RED;
pub const COLOR_AGENT_STARTING: (u8, u8, u8) = COLOR_YELLOW;
pub const COLOR_AGENT_NOT_LOADED: (u8, u8, u8) = COLOR_RED;

// Icon configuration
pub const STATUS_DOT_SIZE: u32 = 10;
pub const STATUS_DOT_OFFSET: u32 = 1; // From bottom-right corner
