use std::pin::Pin;

use chrono::{DateTime, Utc};
use firestore::struct_path::paths;
use firestore::{
    FirestoreDb, FirestoreListenEvent, FirestoreListener, FirestoreMemListenStateStorage,
    FirestoreReference, FirestoreResult,
};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

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

    pub const fn determine_level(&self) -> backend_api::Level {
        // this may seem a bit ridiculous compared to just using ifs, but we want to ensure that EVERY level is completed.
        // In other words, we want to return the first level that hasn't been completed.
        match (
            self.l0_completed(),
            self.l1_completed(),
            self.l2_completed(),
            self.l3_completed(),
            self.l4_completed(),
        ) {
            (true, true, true, true, true) => backend_api::Level::Finish,
            (true, true, true, true, _) => backend_api::Level::L4,
            (true, true, true, _, _) => backend_api::Level::L3,
            (true, true, _, _, _) => backend_api::Level::L2,
            (true, _, _, _, _) => backend_api::Level::L1,
            (_, _, _, _, _) => backend_api::Level::L0,
        }
    }

    pub fn to_api(&self) -> backend_api::GameState {
        backend_api::GameState {
            level: self.determine_level(),
        }
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
