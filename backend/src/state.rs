use std::net::IpAddr;

pub use self::game::Game;
use self::global::Global;
use self::ws_session::WsSession;
pub use self::ws_session::WsSessionKind;
use backend_api as api;
use firestore::{FirestoreDb, FirestoreListenerTarget, FirestoreReference, FirestoreResult};
use futures::Stream;

mod game;
mod global;
mod ws_session;

// listeners are defined here to ensure they are unique across the entire application
const GAME_LISTENER: FirestoreListenerTarget = FirestoreListenerTarget::new(17_u32);

#[derive(Clone)]
pub struct StateHandle {
    db: FirestoreDb,
}

impl StateHandle {
    pub async fn new() -> anyhow::Result<Self> {
        let db = FirestoreDb::new(crate::consts::PROJECT_ID).await?;
        Ok(Self { db })
    }

    pub async fn start_ws_session(
        &self,
        kind: WsSessionKind,
        client_ip: IpAddr,
    ) -> FirestoreResult<String> {
        WsSession::start(&self.db, kind, client_ip).await
    }

    pub async fn end_ws_session(&self, session_id: &str) -> FirestoreResult<()> {
        WsSession::end(&self.db, session_id).await
    }

    pub async fn get_active_game(&self) -> FirestoreResult<FirestoreReference> {
        Global::get_active_game(&self.db).await
    }

    pub async fn stream_game(
        &self,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<impl Stream<Item = Game>> {
        Game::stream(&self.db, game_ref).await
    }

    pub async fn complete_l0(&self, game_ref: &FirestoreReference) -> FirestoreResult<()> {
        Game::complete_l0(&self.db, game_ref).await
    }

    pub async fn complete_l1(&self, game_ref: &FirestoreReference) -> FirestoreResult<()> {
        Game::complete_l1(&self.db, game_ref).await
    }

    pub async fn complete_l2(&self, game_ref: &FirestoreReference) -> FirestoreResult<()> {
        Game::complete_l2(&self.db, game_ref).await
    }

    pub async fn complete_l3(&self, game_ref: &FirestoreReference) -> FirestoreResult<()> {
        Game::complete_l3(&self.db, game_ref).await
    }

    pub async fn perform_labyrinth_action(
        &self,
        game_ref: &FirestoreReference,
        action: api::labyrinth::Action,
    ) -> FirestoreResult<bool> {
        Game::perform_labyrinth_action(&self.db, game_ref, action).await
    }

    pub async fn _complete_l4(&self, game_ref: &FirestoreReference) -> FirestoreResult<()> {
        Game::complete_l4(&self.db, game_ref).await
    }
}
