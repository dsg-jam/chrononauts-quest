use std::net::IpAddr;

use chrono::{DateTime, Utc};
use firestore::{paths, FirestoreDb, FirestoreReference};

mod labyrinth;

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
    ) -> anyhow::Result<String> {
        let obj: WsSession = self
            .db
            .fluent()
            .insert()
            .into(WsSession::COLLECTION)
            .generate_document_id()
            .return_only_fields([] as [&str; 0])
            .object(&WsSession {
                started_at: Some(Utc::now()),
                kind: Some(kind),
                client_ip: Some(client_ip),
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
    kind: Option<WsSessionKind>,
    #[serde(default)]
    client_ip: Option<IpAddr>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum WsSessionKind {
    Board,
    Website,
}

impl WsSessionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Board => "board",
            Self::Website => "website",
        }
    }
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
    const _COLLECTION: &str = "games";
}
