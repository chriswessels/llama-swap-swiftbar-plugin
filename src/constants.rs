// Service configuration
pub const LAUNCH_AGENT_LABEL: &str = "com.user.llama-swap";

// API configuration
pub const API_BASE_URL: &str = "http://127.0.0.1";
pub const API_PORT: u16 = 45786;
pub const API_TIMEOUT_SECS: u64 = 1;

// Update timing
pub const STREAMING_MODE: bool = true;

// Chart configuration
pub const CHART_WIDTH: u32 = 60;
pub const CHART_HEIGHT: u32 = 20;
pub const HISTORY_SIZE: usize = 300; // 5 minutes at 1-second intervals (adaptive polling)
// TODO: ADD TIME BASED CUTOFF HERE

// File paths (using home directory expansion)
pub const LOG_FILE_PATH: &str = "~/Library/Logs/LlamaSwap.log";
pub const CONFIG_FILE_PATH: &str = "~/.llamaswap/config.yaml";

pub const COLOR_TPS_LINE: (u8, u8, u8) = (0, 255, 127);    // Spring green - Generation speed
pub const COLOR_PROMPT_LINE: (u8, u8, u8) = (255, 215, 0); // Gold - Prompt speed
pub const COLOR_MEM_LINE: (u8, u8, u8) = (0, 191, 255);    // Deep sky blue - Memory
pub const COLOR_KV_CACHE_LINE: (u8, u8, u8) = (147, 112, 219); // Medium slate blue - KV cache

// Program state color palette (RGB)
pub const COLOR_BLUE: (u8, u8, u8) = (0, 122, 255);     // Processing/Active
pub const COLOR_GREEN: (u8, u8, u8) = (52, 199, 89);    // Ready/Success
pub const COLOR_YELLOW: (u8, u8, u8) = (255, 149, 0);   // Loading/Starting
pub const COLOR_GREY: (u8, u8, u8) = (142, 142, 147);   // Idle/No Model
pub const COLOR_RED: (u8, u8, u8) = (255, 59, 48);      // Error/Not Loaded

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