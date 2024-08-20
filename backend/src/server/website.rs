use std::pin::pin;

use anyhow::Context;
use backend_api::{GameState, WebMessage};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use super::WebSocketStream;
use crate::state::StateHandle;

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

trait WebsiteStream {
    async fn send_game_state(&mut self, state: GameState) -> anyhow::Result<()> {
        self.send_web_msg(&WebMessage::GameState(state)).await
    }

    async fn recv_board_msg(&mut self) -> Option<WebMessage> {
        loop {
            match self.try_recv_web_msg().await {
                Some(Ok(msg)) => return Some(msg),
                None => return None,
                Some(Err(err)) => {
                    tracing::error!(err = &*err, "failed to receive message");
                }
            }
        }
    }

    async fn send_web_msg(&mut self, msg: &WebMessage) -> anyhow::Result<()>;
    async fn try_recv_web_msg(&mut self) -> Option<anyhow::Result<WebMessage>>;
}

impl WebsiteStream for WebSocketStream {
    async fn send_web_msg(&mut self, msg: &WebMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg)?;
        self.send(Message::Binary(payload)).await?;
        Ok(())
    }

    async fn try_recv_web_msg(&mut self) -> Option<anyhow::Result<WebMessage>> {
        let read = self.by_ref().filter_map(|msg| async move {
            match msg {
                Err(err) => Some(Err(anyhow::Error::from(err))),
                Ok(msg) => {
                    if !(msg.is_binary() || msg.is_text()) {
                        tracing::trace!(?msg, "ignoring non-payload message");
                        return None;
                    }
                    let msg = serde_json::from_slice::<WebMessage>(&msg.into_data());
                    Some(msg.map_err(anyhow::Error::from))
                }
            }
        });
        let mut read = pin!(read);
        read.next().await
    }
}
