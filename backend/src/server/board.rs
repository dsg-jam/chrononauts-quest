use std::pin::pin;

use crate::state::{Game, StateHandle};

use super::WebSocketStream;
use backend_api::{BoardMessage, GameState};
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
            Event::Message(BoardMessage::FrequencyTuned) => {
                state.complete_l2(&game_ref).await?;
            }
            Event::Message(BoardMessage::LabyrinthAction(_action)) => {
                todo!()
            }
            Event::Message(BoardMessage::LogEntry(entry)) => {
                tracing::info!(?entry, "received log entry");
            }
            Event::Message(msg) => {
                tracing::warn!(?msg, "ignoring unexpected message");
            }
            Event::Game(game) => {
                let api_game = game.to_api();
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
    Message(BoardMessage),
    Game(Game),
    Stop,
}

trait BoardMsgStream {
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

impl BoardMsgStream for WebSocketStream {
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
