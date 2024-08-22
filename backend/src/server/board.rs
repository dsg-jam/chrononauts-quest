use std::pin::pin;

use crate::state::{Game, StateHandle};

use super::WebSocketStream;
use backend_api as api;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

pub async fn serve(state: StateHandle, ws_stream: &mut WebSocketStream) -> anyhow::Result<()> {
    let game_ref = state.get_active_game().await?;
    let mut game_stream = state.stream_game(&game_ref).await?;

    let mut last_api_game = None;

    loop {
        let event = tokio::select! {
            msg = ws_stream.recv_board_msg() => msg.map_or(Event::Stop, Event::Message),
            game = game_stream.next() => game.map_or(Event::Stop, Event::Game),
        };
        match event {
            Event::Message(api::BoardMessage::FrequencyTuned) => {
                state.complete_l2(&game_ref).await?;
            }
            Event::Message(api::BoardMessage::LabyrinthAction(action)) => {
                let success = state.perform_labyrinth_action(&game_ref, action).await?;
                if !success {
                    ws_stream.send_labyrinth_action_rejected().await?;
                }
            }
            Event::Message(api::BoardMessage::LogEntry(entry)) => {
                tracing::info!(?entry, "received log entry");
            }
            Event::Message(msg) => {
                tracing::warn!(?msg, "ignoring unexpected message");
            }
            Event::Game(game) => {
                let api_game = game.state_to_api();
                if Some(&api_game) != last_api_game.as_ref() {
                    // api game state has changed, send it to the board
                    last_api_game = Some(api_game.clone());
                    ws_stream.send_game_state(api_game).await?;
                }

                if !game.l1_completed() {
                    // the fact that we have a connection to the board means l1 is complete
                    state.complete_l1(&game_ref).await?;
                }
            }
            Event::Stop => break,
        }
    }
    tracing::warn!("stopping");
    Ok(())
}

enum Event {
    Message(api::BoardMessage),
    Game(Game),
    Stop,
}

trait BoardMsgStream {
    async fn send_game_state(&mut self, state: api::GameState) -> anyhow::Result<()> {
        self.send_board_msg(&api::BoardMessage::GameState(state))
            .await
    }

    async fn send_labyrinth_action_rejected(&mut self) -> anyhow::Result<()> {
        self.send_board_msg(&api::BoardMessage::LabyrinthActionRejected)
            .await
    }

    async fn recv_board_msg(&mut self) -> Option<api::BoardMessage> {
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

    async fn send_board_msg(&mut self, msg: &api::BoardMessage) -> anyhow::Result<()>;
    async fn try_recv_board_msg(&mut self) -> Option<anyhow::Result<api::BoardMessage>>;
}

impl BoardMsgStream for WebSocketStream {
    async fn send_board_msg(&mut self, msg: &api::BoardMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(msg)?;
        self.send(Message::Binary(payload)).await?;
        Ok(())
    }

    async fn try_recv_board_msg(&mut self) -> Option<anyhow::Result<api::BoardMessage>> {
        let read = self.by_ref().filter_map(|msg| async move {
            match msg {
                Err(err) => Some(Err(anyhow::Error::from(err))),
                Ok(msg) => {
                    if !(msg.is_binary() || msg.is_text()) {
                        tracing::trace!(?msg, "ignoring non-payload message");
                        return None;
                    }
                    let msg = serde_json::from_slice::<api::BoardMessage>(&msg.into_data());
                    Some(msg.map_err(anyhow::Error::from))
                }
            }
        });
        let mut read = pin!(read);
        read.next().await
    }
}
