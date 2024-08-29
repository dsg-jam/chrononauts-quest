use std::{ffi::CStr, net::Ipv4Addr};

pub const WEBSOCKET_URI: &str = "wss://api.chrononauts.quest/board";

pub const BOARD_PASSWORD: &str = "d81bc8c90e0ee8db";

pub const L3_ENCODED_KEY: &str = ". -. .. --. -- .-"; // enigma

pub const WINDOW_SIZE: usize = 1;
pub const TIMEOUT_MSEC: usize = 1000;

// Maximum packet size is 61 bytes (64 - 3 bytes for length and RSSI/LQI)
pub const MAX_PACKET_SIZE: usize = 61;
pub const RADIO_FREQUENCY_HZ: u64 = 433_920_000;

/// Wi-Fi
pub const AP_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 9, 1, 1);

/// # Peripherals
/// ## Accelerometer

/// The I2C address of the 3-axis accelerometer
pub const ACCEL_I2C_ADDR: u8 = 0x15;
pub const ACCEL_WHO_AM_I_REG: u8 = 0x0F;
pub const ACCEL_WHO_AM_I_VAL: u8 = 0x5;
pub const ACCEL_ORIENTATION_REG: u8 = 0x1;

/// Orientation/Direction of the accelerometer fetch interval
pub const ACCEL_FETCH_INTERVAL_MS: u64 = 500;

/// ## Potentiometer

/// Sampling rate of the potentiometer in milliseconds
pub const POTI_SAMPLING_RATE_MS: u64 = 100;

/// Event IDs
///
/// These are used to identify the events in the event loop. All events MUST have a unique ID.
///
pub const MAIN_EVENT_BASE: &CStr = c"MAIN_EVENT";
pub const PACKET_RECEPTION_EVENT_BASE: &CStr = c"PR_EVENT";
pub const PACKET_TRANSMISSION_EVENT_BASE: &CStr = c"PT_EVENT";
pub const MESSAGE_TRANSMISSION_EVENT_BASE: &CStr = c"MT_EVENT";
pub const GAME_LOOP_EVENT_BASE: &CStr = c"GL_EVENT";
pub const WS_TRANSMISSION_EVENT_BASE: &CStr = c"WS_TX_EVENT";

pub const MAIN_EVENT_ID: i32 = 0;
pub const PACKET_RECEPTION_EVENT_ID: i32 = 1;
pub const PACKET_TRANSMISSION_EVENT_ID: i32 = 2;
pub const MESSAGE_TRANSMISSION_EVENT_ID: i32 = 3;
pub const GAME_LOOP_EVENT_ID: i32 = 4;
pub const WS_TRANSMISSION_EVENT_ID: i32 = 5;

/// Logger prefixes
pub const CNT_WS_PREFIX: &str = "CHRONONAUTS_WS";
