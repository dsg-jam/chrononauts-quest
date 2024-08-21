use firestore::struct_path::paths; // avoids allocating a vec compared to firestore::paths! macro
use firestore::{FirestoreDb, FirestoreReference, FirestoreResult};
use futures::FutureExt;

use super::Game;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Global {
    #[serde(default)]
    active_game: Option<FirestoreReference>,
}

impl Global {
    const COLLECTION: &str = "global";
    const DOCUMENT_ID: &str = "default";

    pub(super) async fn get_active_game(db: &FirestoreDb) -> FirestoreResult<FirestoreReference> {
        let active_game_ref = db
            .run_transaction(|db, transaction| {
                async move {
                    let active_game_ref = Self::select_active_game_ref(&db).await?;
                    let active_game_ref = match active_game_ref {
                        Some(r) => {
                            tracing::info!("active game found");
                            r
                        }
                        None => {
                            tracing::info!("no active game found, creating a new one");
                            let game_ref = Game::create_new(&db).await?;
                            db.fluent()
                                .update()
                                .fields(paths!(Global::{active_game}))
                                .in_col(Self::COLLECTION)
                                .document_id(Self::DOCUMENT_ID)
                                .object(&Self {
                                    active_game: Some(game_ref.clone()),
                                })
                                .add_to_transaction(transaction)?;
                            game_ref
                        }
                    };
                    Ok(active_game_ref)
                }
                .boxed()
            })
            .await?;
        Ok(active_game_ref)
    }

    async fn select_active_game_ref(
        db: &FirestoreDb,
    ) -> FirestoreResult<Option<FirestoreReference>> {
        let global: Option<Global> = db
            .fluent()
            .select()
            .fields(paths!(Global::{active_game}))
            .by_id_in(Global::COLLECTION)
            .obj()
            .one(Global::DOCUMENT_ID)
            .await?;
        let active_game_ref = global.and_then(|g| g.active_game);
        Ok(active_game_ref)
    }
}
