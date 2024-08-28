//! This module implements the WebSocket client for the Chrononauts board.
//!
//! The WebSocket client is used to connect to the Chrononauts server and send and receive messages.
//!

use core::time::Duration;
use std::{pin::pin, thread};

use backend_api::BoardMessage;
use esp_idf_svc::{
    hal::{delay, task::block_on},
    io::EspIOError,
    sys::EspError,
    tls::X509,
    ws::client::{
        EspWebSocketClient, EspWebSocketClientConfig, FrameType, WebSocketEvent, WebSocketEventType,
    },
};

use log::info;

use crate::{
    consts::{self, CNT_WS_PREFIX},
    event::{MainEvent, WsTransmissionEvent},
    radio::{ChrononautsMessage, MessageError},
    ChrononautsEventLoop,
};

#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error(transparent)]
    EspIOError(#[from] EspIOError),
    #[error(transparent)]
    EspError(#[from] EspError),
    #[error(transparent)]
    MessageError(#[from] MessageError),
    #[error("WebSocket not connected")]
    WsNotConnected,
}

pub struct ChrononautsWebSocketClient(EspWebSocketClient<'static>);

impl ChrononautsWebSocketClient {
    pub fn new(event_loop: ChrononautsEventLoop) -> Self {
        let config = EspWebSocketClientConfig {
            server_cert: Some(X509::pem_until_nul(consts::SERVER_ROOT_CERT)),
            ..Default::default()
        };
        let timeout = Duration::from_secs(10);
        let uri = format!(
            "{}?password={}",
            consts::WEBSOCKET_URI,
            consts::BOARD_PASSWORD
        );
        let client = EspWebSocketClient::new(&uri, &config, timeout, move |event| {
            handle_event(event_loop.clone(), event)
        })
        .unwrap();
        Self(client)
    }

    pub fn is_connected(&self) -> bool {
        self.0.is_connected()
    }

    pub fn send_message(&mut self, payload: &[u8]) {
        self.0.send(FrameType::Text(false), payload).unwrap();
    }
}

fn handle_event(event_loop: ChrononautsEventLoop, event: &Result<WebSocketEvent, EspIOError>) {
    if let Ok(event) = event {
        match event.event_type {
            WebSocketEventType::BeforeConnect => {
                info!("Websocket before connect");
            }
            WebSocketEventType::Connected => {
                info!("Websocket connected");
            }
            WebSocketEventType::Disconnected => {
                info!("Websocket disconnected");
            }
            WebSocketEventType::Close(reason) => {
                info!("Websocket close, reason: {reason:?}");
            }
            WebSocketEventType::Closed => {
                info!("Websocket closed");
            }
            WebSocketEventType::Text(text) => {
                // Backend ALWAYS ONLY sends binary messages, so this is not used
                // But, log the message in case it is received
                log::error!("[{CNT_WS_PREFIX}] Unexpected text message: {text}");
            }
            WebSocketEventType::Binary(binary) => {
                // Backend ALWAYS ONLY sends binary messages representing a BoardMessage
                let res = serde_json::from_slice::<BoardMessage>(binary);
                let Ok(board_msg) = res else {
                    // Otherwise, ignore the message & log the error
                    log::error!("[{CNT_WS_PREFIX}] Failed to parse binary message: {res:?}");
                    return;
                };
                let Ok(msg) = ChrononautsMessage::try_from(board_msg) else {
                    log::error!(
                        "[{CNT_WS_PREFIX}] Failed to convert BoardMessage to ChrononautsMessage."
                    );
                    return;
                };
                event_loop
                    .post::<MainEvent>(&MainEvent::MessageReceived(msg), delay::BLOCK)
                    .unwrap();
            }
            WebSocketEventType::Ping => {
                info!("Websocket ping");
            }
            WebSocketEventType::Pong => {
                info!("Websocket pong");
            }
        }
    }
}

pub type WsHandler = thread::JoinHandle<Result<(), WsError>>;

pub fn run(
    mut ws_client: ChrononautsWebSocketClient,
    event_loop: ChrononautsEventLoop,
) -> Result<WsHandler, WsError> {
    if !ws_client.is_connected() {
        return Err(WsError::WsNotConnected);
    }
    let handler = thread::spawn(move || {
        block_on(pin!(async move {
            let mut subscription = event_loop.subscribe_async::<WsTransmissionEvent>()?;

            loop {
                let event = subscription.recv().await?;
                let WsTransmissionEvent::Send(msg) = event;

                let board_msg = BoardMessage::try_from(msg)?;
                ws_client.send_message(&serde_json::to_vec(&board_msg).unwrap());
            }
        }))
    });
    Ok(handler)
}
