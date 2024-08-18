use std::net::IpAddr;

use chrono::{DateTime, Utc};
use firestore::{paths, FirestoreDb, FirestoreMemListenStateStorage, FirestoreReference};

#[derive(Clone)]
pub struct State {
    db: FirestoreDb,
}

impl State {
    pub async fn new() -> anyhow::Result<Self> {
        let db = FirestoreDb::new(crate::consts::PROJECT_ID).await?;
        db.create_listener(FirestoreMemListenStateStorage::new());
        Ok(Self { db })
    }

    pub async fn start_ws_session(&self, peer_ip: IpAddr) -> anyhow::Result<String> {
        let obj: WsSession = self
            .db
            .fluent()
            .insert()
            .into(WsSession::COLLECTION)
            .generate_document_id()
            .return_only_fields([] as [&str; 0])
            .object(&WsSession {
                started_at: Some(Utc::now()),
                peer_ip: Some(peer_ip),
                ..Default::default()
            })
            .execute()
            .await?;
        Ok(obj.id.unwrap())
    }

    pub async fn end_ws_session(&self, session_id: &str) -> anyhow::Result<()> {
        let _obj: WsSession = self
            .db
            .fluent()
            .update()
            .fields(paths!(WsSession::{ended_at}))
            .in_col(WsSession::COLLECTION)
            .return_only_fields([] as [&str; 0])
            .document_id(session_id)
            .object(&WsSession {
                ended_at: Some(Utc::now()),
                ..Default::default()
            })
            .execute()
            .await?;
        Ok(())
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Global {
    #[serde(default)]
    active_game: Option<FirestoreReference>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct WsSession {
    #[serde(alias = "_firestore_id")]
    id: Option<String>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    started_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    ended_at: Option<DateTime<Utc>>,
    #[serde(default)]
    peer_ip: Option<IpAddr>,
}

impl WsSession {
    const COLLECTION: &str = "ws-sessions";
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Game {
    #[serde(alias = "_firestore_id")]
    id: Option<String>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    started_at: Option<DateTime<Utc>>,
    #[serde(default, with = "firestore::serialize_as_optional_timestamp")]
    ended_at: Option<DateTime<Utc>>,
}

impl Game {
    const COLLECTION: &str = "games";
}
