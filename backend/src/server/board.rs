use std::pin::pin;

use crate::state::StateHandle;

use super::WebSocketStream;
use anyhow::Context;
use backend_api::{BoardMessage, GameState};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

pub async fn serve(_state: StateHandle, ws_stream: &mut WebSocketStream) -> anyhow::Result<()> {
    ws_stream
        .send_game_state(GameState {
            level: backend_api::Level::L0,
        })
        .await
        .context("failed to send initial game state")?;

    while let Some(msg) = ws_stream.recv_board_msg().await {
        tracing::debug!("{msg:?}");
    }

    Ok(())
}

trait BoardStream {
    async fn send_game_state(&mut self, state: GameState) -> anyhow::Result<()> {
        self.send_board_msg(&BoardMessage::GameState(state)).await
    }
    async fn recv_board_msg(&mut self) -> Option<BoardMessage> {
        loop {
            match self.try_recv_board_msg().await {
                Some(Ok(msg)) => return Some(msg),
                None => return None,
                Some(Err(err)) => {
                    tracing::error!(err = &*err, "failed to receive message");
                }
            }
        }
    }

    async fn send_board_msg(&mut self, msg: &BoardMessage) -> anyhow::Result<()>;
    async fn try_recv_board_msg(&mut self) -> Option<anyhow::Result<BoardMessage>>;
}

impl BoardStream for WebSocketStream {
    async fn send_board_msg(&mut self, msg: &BoardMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg)?;
        self.send(Message::Binary(payload)).await?;
        Ok(())
    }

    async fn try_recv_board_msg(&mut self) -> Option<anyhow::Result<BoardMessage>> {
        let read = self.by_ref().filter_map(|msg| async move {
            match msg {
                Err(err) => Some(Err(anyhow::Error::from(err))),
                Ok(msg) => {
                    if !(msg.is_binary() || msg.is_text()) {
                        tracing::trace!(?msg, "ignoring non-payload message");
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
