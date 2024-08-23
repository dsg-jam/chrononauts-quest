use std::pin::pin;

use backend_api as api;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use super::WebSocketStream;
use crate::state::{Game, StateHandle};

pub async fn serve(state: StateHandle, ws_stream: &mut WebSocketStream) -> anyhow::Result<()> {
    let game_ref = state.get_active_game().await?;
    let mut game_stream = state.stream_game(&game_ref).await?;

    let mut last_api_game = None;
    let mut last_api_labyrinth = None;

    loop {
        let event = tokio::select! {
            msg = ws_stream.recv_web_msg() => msg.map_or(Event::Stop, Event::Message),
            game = game_stream.next() => game.map_or(Event::Stop, Event::Game),
        };
        match event {
            Event::Message(api::WebMessage::EnterEncryptionKey { key }) => {
                tracing::info!(key, "guessed encryption key");
                if key == crate::consts::L3_ENCRYPTION_KEY {
                    state.complete_l3(&game_ref).await?;
                } else {
                    ws_stream.send_encryption_key_rejected().await?;
                }
            }
            Event::Message(msg) => {
                tracing::warn!(?msg, "ignoring unexpected message");
            }
            Event::Game(game) => {
                let api_game = game.state_to_api();
                if Some(&api_game) != last_api_game.as_ref() {
                    // api game state has changed, send it to the website
                    last_api_game = Some(api_game.clone());
                    ws_stream.send_game_state(api_game).await?;
                }
                if let Some(api_labyrinth) = game.labyrinth_to_api() {
                    if Some(&api_labyrinth) != last_api_labyrinth.as_ref() {
                        // api labyrinth state has changed, send it to the website
                        last_api_labyrinth = Some(api_labyrinth.clone());
                        ws_stream.send_labyrinth_state(api_labyrinth).await?;
                    }
                }

                if !game.l0_completed() {
                    // the fact that we have a connection to the website means l0 is complete
                    state.complete_l0(&game_ref).await?;
                }
            }
            Event::Stop => break,
        }
    }
    tracing::warn!("stopping");
    Ok(())
}

enum Event {
    Message(api::WebMessage),
    Game(Game),
    Stop,
}

trait WebMsgStream {
    async fn send_game_state(&mut self, state: api::GameState) -> anyhow::Result<()> {
        self.send_web_msg(&api::WebMessage::GameState(state)).await
    }

    async fn send_labyrinth_state(
        &mut self,
        state: api::labyrinth::FullState,
    ) -> anyhow::Result<()> {
        self.send_web_msg(&api::WebMessage::LabyrinthState(state))
            .await
    }

    async fn send_encryption_key_rejected(&mut self) -> anyhow::Result<()> {
        self.send_web_msg(&api::WebMessage::EncryptionKeyRejected)
            .await
    }

    async fn recv_web_msg(&mut self) -> Option<api::WebMessage> {
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

    async fn send_web_msg(&mut self, msg: &api::WebMessage) -> anyhow::Result<()>;
    async fn try_recv_web_msg(&mut self) -> Option<anyhow::Result<api::WebMessage>>;
}

impl WebMsgStream for WebSocketStream {
    async fn send_web_msg(&mut self, msg: &api::WebMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg)?;
        self.send(Message::Binary(payload)).await?;
        Ok(())
    }

    async fn try_recv_web_msg(&mut self) -> Option<anyhow::Result<api::WebMessage>> {
        let read = self.by_ref().filter_map(|msg| async move {
            match msg {
                Err(err) => Some(Err(anyhow::Error::from(err))),
                Ok(msg) => {
                    if !(msg.is_binary() || msg.is_text()) {
                        tracing::trace!(?msg, "ignoring non-payload message");
                        return None;
                    }
                    let msg = serde_json::from_slice::<api::WebMessage>(&msg.into_data());
                    Some(msg.map_err(anyhow::Error::from))
                }
            }
        });
        let mut read = pin!(read);
        read.next().await
    }
}
