use std::{ffi::CStr, net::Ipv4Addr};

pub const WEBSOCKET_URI: &str = "wss://api.chrononauts.quest/board";

pub const BOARD_PASSWORD: &str = "d81bc8c90e0ee8db";

/// The PEM-encoded GTS Root R1 / GlobalSign Root CA certificate at the end of the cert chain
/// for the websocket server at api.chrononauts.quest
/// This certificate is valid FROM Jun 19 00:00:42 2020 GMT
/// This certificate is NOT valid AFTER Jan 28 00:00:42 2028 GMT
pub const SERVER_ROOT_CERT: &[u8] = b"
-----BEGIN CERTIFICATE-----
MIIFYjCCBEqgAwIBAgIQd70NbNs2+RrqIQ/E8FjTDTANBgkqhkiG9w0BAQsFADBX
MQswCQYDVQQGEwJCRTEZMBcGA1UEChMQR2xvYmFsU2lnbiBudi1zYTEQMA4GA1UE
CxMHUm9vdCBDQTEbMBkGA1UEAxMSR2xvYmFsU2lnbiBSb290IENBMB4XDTIwMDYx
OTAwMDA0MloXDTI4MDEyODAwMDA0MlowRzELMAkGA1UEBhMCVVMxIjAgBgNVBAoT
GUdvb2dsZSBUcnVzdCBTZXJ2aWNlcyBMTEMxFDASBgNVBAMTC0dUUyBSb290IFIx
MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEAthECix7joXebO9y/lD63
ladAPKH9gvl9MgaCcfb2jH/76Nu8ai6Xl6OMS/kr9rH5zoQdsfnFl97vufKj6bwS
iV6nqlKr+CMny6SxnGPb15l+8Ape62im9MZaRw1NEDPjTrETo8gYbEvs/AmQ351k
KSUjB6G00j0uYODP0gmHu81I8E3CwnqIiru6z1kZ1q+PsAewnjHxgsHA3y6mbWwZ
DrXYfiYaRQM9sHmklCitD38m5agI/pboPGiUU+6DOogrFZYJsuB6jC511pzrp1Zk
j5ZPaK49l8KEj8C8QMALXL32h7M1bKwYUH+E4EzNktMg6TO8UpmvMrUpsyUqtEj5
cuHKZPfmghCN6J3Cioj6OGaK/GP5Afl4/Xtcd/p2h/rs37EOeZVXtL0m79YB0esW
CruOC7XFxYpVq9Os6pFLKcwZpDIlTirxZUTQAs6qzkm06p98g7BAe+dDq6dso499
iYH6TKX/1Y7DzkvgtdizjkXPdsDtQCv9Uw+wp9U7DbGKogPeMa3Md+pvez7W35Ei
Eua++tgy/BBjFFFy3l3WFpO9KWgz7zpm7AeKJt8T11dleCfeXkkUAKIAf5qoIbap
sZWwpbkNFhHax2xIPEDgfg1azVY80ZcFuctL7TlLnMQ/0lUTbiSw1nH69MG6zO0b
9f6BQdgAmD06yK56mDcYBZUCAwEAAaOCATgwggE0MA4GA1UdDwEB/wQEAwIBhjAP
BgNVHRMBAf8EBTADAQH/MB0GA1UdDgQWBBTkrysmcRorSCeFL1JmLO/wiRNxPjAf
BgNVHSMEGDAWgBRge2YaRQ2XyolQL30EzTSo//z9SzBgBggrBgEFBQcBAQRUMFIw
JQYIKwYBBQUHMAGGGWh0dHA6Ly9vY3NwLnBraS5nb29nL2dzcjEwKQYIKwYBBQUH
MAKGHWh0dHA6Ly9wa2kuZ29vZy9nc3IxL2dzcjEuY3J0MDIGA1UdHwQrMCkwJ6Al
oCOGIWh0dHA6Ly9jcmwucGtpLmdvb2cvZ3NyMS9nc3IxLmNybDA7BgNVHSAENDAy
MAgGBmeBDAECATAIBgZngQwBAgIwDQYLKwYBBAHWeQIFAwIwDQYLKwYBBAHWeQIF
AwMwDQYJKoZIhvcNAQELBQADggEBADSkHrEoo9C0dhemMXoh6dFSPsjbdBZBiLg9
NR3t5P+T4Vxfq7vqfM/b5A3Ri1fyJm9bvhdGaJQ3b2t6yMAYN/olUazsaL+yyEn9
WprKASOshIArAoyZl+tJaox118fessmXn1hIVw41oeQa1v1vg4Fv74zPl6/AhSrw
9U5pCZEt4Wi4wStz6dTZ/CLANx8LZh1J7QJVj2fhMtfTJr9w4z30Z209fOU0iOMy
+qduBmpvvYuR7hZL6Dupszfnw0Skfths18dG9ZKb59UhvmaSGZRVbNQpsg3BZlvi
d0lIKO2d1xozclOzgjXPYovJJIultzkMu34qQb9Sz/yilrbCgj8=
-----END CERTIFICATE-----\0";

pub const WINDOW_SIZE: usize = 4;
pub const TIMEOUT_MSEC: usize = 2000;

// Maximum packet size is 61 bytes (64 - 3 bytes for length and RSSI/LQI)
pub const MAX_PACKET_SIZE: usize = 61;
pub const RADIO_FREQUENCY_HZ: u64 = 433_920_000;

/// Wi-Fi
pub const AP_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 9, 1, 1);

/// # Peripherals
///
/// ## Accelerometer

/// The I2C address of the 3-axis accelerometer
pub const ACCEL_I2C_ADDR: u8 = 0x15;
pub const ACCEL_WHO_AM_I_REG: u8 = 0x0F;
pub const ACCEL_WHO_AM_I_VAL: u8 = 0x5;
pub const ACCEL_ORIENTATION_REG: u8 = 0x1;

/// Orientation/Direction of the accelerometer fetch interval
pub const ACCEL_FETCH_INTERVAL_MS: u64 = 500;

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
