// Service configuration
pub const LAUNCH_AGENT_LABEL: &str = "com.user.llama-swap";

// API configuration
pub const API_BASE_URL: &str = "http://127.0.0.1";
pub const API_PORT: u16 = 45786;
pub const API_TIMEOUT_SECS: u64 = 1;

// Update timing
pub const UPDATE_INTERVAL_SECS: u64 = 5;     // Default/idle interval
pub const ACTIVE_INTERVAL_SECS: u64 = 1;     // When processing requests
pub const STARTING_INTERVAL_SECS: u64 = 2;   // During state transitions
pub const STREAMING_MODE: bool = true;

// Adaptive polling configuration
pub const MIN_STARTING_DURATION_SECS: u64 = 10;  // Minimum time in Starting mode

// Chart configuration
pub const CHART_WIDTH: u32 = 60;
pub const CHART_HEIGHT: u32 = 15;
pub const HISTORY_SIZE: usize = 300; // 5 minutes at 1-second intervals (adaptive polling)

// File paths (using home directory expansion)
pub const LOG_FILE_PATH: &str = "~/Library/Logs/LlamaSwap.log";
pub const CONFIG_FILE_PATH: &str = "~/.llamaswap/config.yaml";

// Colors (RGB)
pub const COLOR_RUNNING: (u8, u8, u8) = (0, 200, 83);      // Green
pub const COLOR_STOPPED: (u8, u8, u8) = (213, 0, 0);       // Red
pub const COLOR_TPS_LINE: (u8, u8, u8) = (0, 255, 127);    // Spring green - Generation speed
pub const COLOR_PROMPT_LINE: (u8, u8, u8) = (255, 215, 0); // Gold - Prompt speed
pub const COLOR_MEM_LINE: (u8, u8, u8) = (0, 191, 255);    // Deep sky blue - Memory
pub const COLOR_KV_CACHE_LINE: (u8, u8, u8) = (147, 112, 219); // Medium slate blue - KV cache

// Icon configuration
pub const STATUS_DOT_SIZE: u32 = 8;
pub const STATUS_DOT_OFFSET: u32 = 3; // From bottom-right corner