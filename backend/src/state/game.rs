use std::pin::Pin;

use backend_api as api;
use chrono::{DateTime, Utc};
use firestore::struct_path::paths;
use firestore::{
    FirestoreDb, FirestoreListenEvent, FirestoreListener, FirestoreMemListenStateStorage,
    FirestoreReference, FirestoreResult,
};
use futures::{FutureExt, Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::labyrinth::LabyrinthMap;

use super::GAME_LISTENER;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Game {
    #[serde(alias = "_firestore_id")]
    id: Option<String>,
    #[serde(default)]
    backend_version: Option<String>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    started_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    ended_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    l0_completed_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    l1_completed_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    l2_completed_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    l3_completed_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    l4_completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    board1_connected: bool,
    #[serde(default)]
    board2_connected: bool,
    #[serde(default)]
    labyrinth: Labyrinth,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Labyrinth {
    #[serde(default)]
    pub player1: Option<api::labyrinth::PlayerState>,
    #[serde(default)]
    pub player2: Option<api::labyrinth::PlayerState>,
    #[serde(default)]
    map: LabyrinthMap,
}

impl Game {
    pub const fn l0_completed(&self) -> bool {
        self.l0_completed_at.is_some()
    }

    pub const fn l1_completed(&self) -> bool {
        self.l1_completed_at.is_some()
    }

    pub const fn l2_completed(&self) -> bool {
        self.l2_completed_at.is_some()
    }

    pub const fn l3_completed(&self) -> bool {
        self.l3_completed_at.is_some()
    }

    pub const fn l4_completed(&self) -> bool {
        self.l4_completed_at.is_some()
    }

    pub const fn determine_level(&self) -> api::Level {
        // this may seem a bit ridiculous compared to just using ifs, but we want to ensure that EVERY level is completed.
        // In other words, we want to return the first level that hasn't been completed.
        match (
            self.l0_completed(),
            self.l1_completed(),
            self.l2_completed(),
            self.l3_completed(),
            self.l4_completed(),
        ) {
            (true, true, true, true, true) => api::Level::Finish,
            (true, true, true, true, _) => api::Level::L4,
            (true, true, true, _, _) => api::Level::L3,
            (true, true, _, _, _) => api::Level::L2,
            (true, _, _, _, _) => api::Level::L1,
            (_, _, _, _, _) => api::Level::L0,
        }
    }

    pub fn state_to_api(&self) -> api::GameState {
        api::GameState {
            level: self.determine_level(),
        }
    }

    pub fn labyrinth_to_api(&self) -> Option<api::labyrinth::FullState> {
        let player1 = self.labyrinth.player1.clone()?;
        let player2 = self.labyrinth.player2.clone()?;
        Some(api::labyrinth::FullState { player1, player2 })
    }
}

impl Game {
    const COLLECTION: &str = "games";

    pub(super) async fn create_new(db: &FirestoreDb) -> FirestoreResult<FirestoreReference> {
        let game: Game = db
            .fluent()
            .insert()
            .into(Game::COLLECTION)
            .generate_document_id()
            .return_only_fields([] as [&str; 0])
            .object(&Game {
                backend_version: Some(crate::consts::VERSION.to_owned()),
                started_at: Some(Utc::now()),
                ..Default::default()
            })
            .execute()
            .await?;
        let id = game.id.unwrap();
        db.parent_path(Self::COLLECTION, id).map(Into::into)
    }

    pub(super) async fn update_fields(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
        fields: &[&str],
        obj: &Self,
    ) -> FirestoreResult<()> {
        let (_, _, doc_id) = game_ref.split(db.get_documents_path());
        let _game: Self = db
            .fluent()
            .update()
            .fields(fields)
            .in_col(Self::COLLECTION)
            .return_only_fields([] as [&str; 0])
            .document_id(doc_id)
            .object(obj)
            .execute()
            .await?;
        Ok(())
    }

    pub(super) async fn stream(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<impl Stream<Item = Game>> {
        let mut listener = db
            .create_listener(FirestoreMemListenStateStorage::new())
            .await?;

        let (_, _, doc_id) = game_ref.split(db.get_documents_path());
        db.fluent()
            .select()
            .by_id_in(Game::COLLECTION)
            .batch_listen([doc_id])
            .add_target(GAME_LISTENER, &mut listener)?;

        let (tx, rx) = mpsc::channel(1);
        listener
            .start(move |event| {
                let tx = tx.clone();
                async move {
                    if let FirestoreListenEvent::DocumentChange(change) = event {
                        if let Some(doc) = change.document {
                            let game: Game = FirestoreDb::deserialize_doc_to(&doc)?;
                            tx.send(game).await.ok();
                        }
                    }
                    Ok(())
                }
            })
            .await?;
        Ok(GameStateStream::new(rx, listener))
    }

    pub(super) async fn complete_l0(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{l0_completed_at}).as_slice(),
            &Game {
                l0_completed_at: Some(Utc::now()),
                ..Default::default()
            },
        )
        .await
    }

    pub(super) async fn complete_l1(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{l1_completed_at}).as_slice(),
            &Game {
                l1_completed_at: Some(Utc::now()),
                ..Default::default()
            },
        )
        .await
    }

    pub(super) async fn complete_l2(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{l2_completed_at}).as_slice(),
            &Game {
                l2_completed_at: Some(Utc::now()),
                ..Default::default()
            },
        )
        .await
    }

    pub(super) async fn complete_l3(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{l3_completed_at}).as_slice(),
            &Game {
                l3_completed_at: Some(Utc::now()),
                ..Default::default()
            },
        )
        .await
    }

    fn mutate_on_labyrinth_action(&mut self, action: &api::labyrinth::Action) -> bool {
        let player_state = {
            // make sure both players are initialized
            let p1 = self
                .labyrinth
                .player1
                .get_or_insert(self.labyrinth.map.player1_start_state.clone());
            let p2 = self
                .labyrinth
                .player2
                .get_or_insert(self.labyrinth.map.player2_start_state.clone());
            match action.device {
                api::DeviceId::Player1 => p1,
                api::DeviceId::Player2 => p2,
            }
        };
        player_state.direction = action.direction;
        if !action.step {
            // if we don't want to move, we're done
            return true;
        }

        let pos = &mut player_state.position;
        let Some(new_pos) = self.labyrinth.map.try_move(pos.clone(), action.direction) else {
            return false;
        };
        *pos = new_pos;
        true
    }

    pub(super) async fn labyrinth_solved(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<bool> {
        let (_, _, doc_id) = game_ref.split(db.get_documents_path());
        let game: Option<Game> = db
            .fluent()
            .select()
            .fields(paths!(Game::{labyrinth}))
            .by_id_in(Game::COLLECTION)
            .obj()
            .one(doc_id.clone())
            .await?;
        let Some(mut game) = game else {
            tracing::warn!("game not found");
            return Ok(false);
        };
        let p1 = game
            .labyrinth
            .player1
            .get_or_insert(game.labyrinth.map.player1_start_state.clone());
        let p2 = game
            .labyrinth
            .player2
            .get_or_insert(game.labyrinth.map.player2_start_state.clone());
        if p1.position == p2.position {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(super) async fn perform_labyrinth_action(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
        action: api::labyrinth::Action,
    ) -> FirestoreResult<bool> {
        let (_, _, doc_id) = game_ref.split(db.get_documents_path());
        db.run_transaction(move |db, transaction| {
            let doc_id = doc_id.clone();
            let action = action.clone();
            async move {
                let game: Option<Game> = db
                    .fluent()
                    .select()
                    .fields(paths!(Game::{labyrinth}))
                    .by_id_in(Game::COLLECTION)
                    .obj()
                    .one(doc_id.clone())
                    .await?;
                let Some(mut game) = game else {
                    tracing::warn!("game not found");
                    return Ok(false);
                };
                let success = game.mutate_on_labyrinth_action(&action);
                if !success {
                    return Ok(false);
                }
                db.fluent()
                    .update()
                    .fields(paths!(Game::{labyrinth}))
                    .in_col(Game::COLLECTION)
                    .document_id(doc_id)
                    .object(&game)
                    .add_to_transaction(transaction)?;
                Ok(true)
            }
            .boxed()
        })
        .await
    }

    pub(super) async fn complete_l4(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{l4_completed_at}).as_slice(),
            &Game {
                l4_completed_at: Some(Utc::now()),
                ..Default::default()
            },
        )
        .await
    }

    pub(super) async fn set_board1_connected(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
        connected: bool,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{board1_connected}).as_slice(),
            &Game {
                board1_connected: connected,
                ..Default::default()
            },
        )
        .await
    }

    pub(super) async fn set_board2_connected(
        db: &FirestoreDb,
        game_ref: &FirestoreReference,
        connected: bool,
    ) -> FirestoreResult<()> {
        Self::update_fields(
            db,
            game_ref,
            paths!(Game::{board2_connected}).as_slice(),
            &Game {
                board2_connected: connected,
                ..Default::default()
            },
        )
        .await
    }
}

struct GameStateStream {
    inner: ReceiverStream<Game>,
    _listener: FirestoreListener<FirestoreDb, FirestoreMemListenStateStorage>,
}

impl GameStateStream {
    fn new(
        rx: mpsc::Receiver<Game>,
        listener: FirestoreListener<FirestoreDb, FirestoreMemListenStateStorage>,
    ) -> Self {
        Self {
            inner: ReceiverStream::new(rx),
            _listener: listener,
        }
    }
}

impl Stream for GameStateStream {
    type Item = Game;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.get_mut().inner.poll_next_unpin(cx)
    }
}
