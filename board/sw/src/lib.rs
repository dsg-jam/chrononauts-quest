mod captive;
mod communication;
pub mod consts;
mod dns;
mod event;
mod game_loop;
mod http_server;
mod peripherals;
mod radio;
mod utils;
mod wifi;
mod ws;

use std::sync::{Arc, Mutex};

use esp_idf_svc::{
    eventloop::{Background, EspEventLoop, User},
    sys::EspError,
    wifi::AccessPointInfo,
};
pub use event::MainEvent;
pub use game_loop::GameLoop;
pub use http_server::ChrononautsHttpServer;
pub use peripherals::{
    ChrononautsAccelerometer, ChrononautsLed, ChrononautsPotentiometer, PeripheralError,
};
pub use wifi::{ChrononautsWifi, WifiRunner};

pub use captive::CaptivePortal;
pub use dns::SimpleDns;
pub use radio::{ChrononautsTransceiver, ChrononautsTransport};
pub use utils::{get_chrononauts_id, ChrononautsId};
pub use ws::ChrononautsWebSocketClient;
use ws::WsError;

pub type ChrononautsEventLoop = EspEventLoop<User<Background>>;
pub type ChrononautsSSIDs = Arc<Mutex<Vec<AccessPointInfo>>>;

#[derive(Debug, thiserror::Error)]
pub enum ChrononautsError {
    #[error("Invalid Chrononauts ID")]
    InvalidChrononautsId,
    #[error(transparent)]
    EspError(#[from] EspError),
    #[error(transparent)]
    PeripheralError(#[from] PeripheralError),
    #[error(transparent)]
    WsError(#[from] WsError),
}

#[allow(dead_code)]
// Required for rust-analyzer to not throw an error
fn main() {}
