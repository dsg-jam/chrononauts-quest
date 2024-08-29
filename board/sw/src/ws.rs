//! This module implements the WebSocket client for the Chrononauts board.
//!
//! The WebSocket client is used to connect to the Chrononauts server and send and receive messages.
//!

use core::time::Duration;
use std::pin::pin;

use backend_api::BoardMessage;
use esp_idf_svc::{
    hal::{delay, task::block_on},
    io::EspIOError,
    sys::EspError,
    ws::client::{
        EspWebSocketClient, EspWebSocketClientConfig, FrameType, WebSocketEvent, WebSocketEventType,
    },
};

use log::info;

use crate::{
    communication::{ChrononautsMessage, MessageError},
    consts::{self, CNT_WS_PREFIX},
    event::{MainEvent, WsTransmissionEvent},
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

pub struct ChrononautsWebSocketClient {
    client: Option<EspWebSocketClient<'static>>,
    event_loop: ChrononautsEventLoop,
}

impl ChrononautsWebSocketClient {
    pub fn new(event_loop: ChrononautsEventLoop) -> Self {
        Self {
            client: None,
            event_loop,
        }
    }

    fn connect(&mut self) -> Result<(), WsError> {
        let config = EspWebSocketClientConfig::default();
        let timeout = Duration::from_secs(5);
        let uri = format!(
            "{}?password={}",
            consts::WEBSOCKET_URI,
            consts::BOARD_PASSWORD
        );
        let event_loop = self.event_loop.clone();
        let client = EspWebSocketClient::new(&uri, &config, timeout, move |event| {
            handle_event(event_loop.clone(), event)
        })?;
        self.client = Some(client);

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        if self.client.is_none() {
            return false;
        }
        self.client.as_ref().unwrap().is_connected()
    }

    pub fn send_message(&mut self, payload: &[u8]) -> Result<(), WsError> {
        if !self.is_connected() {
            return Err(WsError::WsNotConnected);
        }
        self.client
            .as_mut()
            .unwrap()
            .send(FrameType::Text(false), payload)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), WsError> {
        block_on(pin!(async move {
            let mut subscription = self.event_loop.subscribe_async::<WsTransmissionEvent>()?;
            loop {
                let event = subscription.recv().await?;

                match event {
                    WsTransmissionEvent::Send(msg) => {
                        let Ok(board_msg) = BoardMessage::try_from(msg) else {
                            log::error!("[{CNT_WS_PREFIX}] Failed to convert ChrononautsMessage to BoardMessage.");
                            continue;
                        };
                        self.send_message(&serde_json::to_vec(&board_msg).unwrap())?;
                    }
                    WsTransmissionEvent::Connect => {
                        self.connect()?;
                    }
                }
            }
        }))
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
