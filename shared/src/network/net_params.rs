pub const SECRET_KEY_SIZE: usize = 32; // if changed, change protocol version
pub const A_B_BITS: u64 = 2048; // if changed, change protocol version
pub const IV_LENGTH: usize = 16; // if changed, change protocol version
pub const CMAC_TAG_LENGTH: usize = 16; // if changed, change protocol version

pub const MAX_WAIT_TIME_IN_READ_WRITE_MS: u64 = 120000; // 2 minutes
pub const WOULD_BLOCK_RETRY_AFTER_MS: u64 = 20;

pub const NETWORK_PROTOCOL_VERSION: u16 = 2;
