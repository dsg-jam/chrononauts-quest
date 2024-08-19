use std::pin::pin;

use anyhow::Context;
use backend_api::{BoardMessage, GameState};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::State;

pub async fn serve(_state: State, ws_stream: WebSocketStream<TcpStream>) -> anyhow::Result<()> {
    let mut board_socket = BoardSocket::new(ws_stream);
    board_socket
        .send_game_state(GameState {
            level: backend_api::Level::L0,
        })
        .await
        .context("failed to send initial game state")?;

    while let Some(msg) = board_socket.recv().await {
        tracing::debug!("{msg:?}");
    }

    Ok(())
}

struct BoardSocket {
    ws_stream: WebSocketStream<TcpStream>,
}

impl BoardSocket {
    fn new(ws_stream: WebSocketStream<TcpStream>) -> Self {
        Self { ws_stream }
    }

    async fn send_game_state(&mut self, state: GameState) -> anyhow::Result<()> {
        self.send(&BoardMessage::GameState(state)).await
    }

    async fn send(&mut self, msg: &BoardMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg)?;
        self.ws_stream.send(Message::Binary(payload)).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Option<BoardMessage> {
        loop {
            match self.try_recv().await {
                Some(Ok(msg)) => return Some(msg),
                None => return None,
                Some(Err(err)) => {
                    tracing::error!(err = &*err, "failed to receive message");
                }
            }
        }
    }

    async fn try_recv(&mut self) -> Option<anyhow::Result<BoardMessage>> {
        let read = self.ws_stream.by_ref().filter_map(|msg| async move {
            match msg {
                Err(err) => Some(Err(anyhow::Error::from(err))),
                Ok(msg) => {
                    if !(msg.is_binary() || msg.is_text()) {
                        return None;
                    }
                    let msg = serde_json::from_slice::<BoardMessage>(&msg.into_data());
                    Some(msg.map_err(anyhow::Error::from))
                }
            }
        });
        let mut read = pin!(read);
        read.next().await
    }
}
