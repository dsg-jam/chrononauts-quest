use chrono::{DateTime, Utc};
use firestore::struct_path::paths; // avoids allocating a vec compared to firestore::paths! macro
use firestore::{FirestoreDb, FirestoreResult};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WsSession {
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

impl WsSession {
    const COLLECTION: &str = "ws-sessions";

    pub(super) async fn start(
        db: &FirestoreDb,
        kind: WsSessionKind,
        client_ip: IpAddr,
    ) -> FirestoreResult<String> {
        let sess: WsSession = db
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
        Ok(sess.id.unwrap())
    }

    pub(super) async fn end(db: &FirestoreDb, session_id: &str) -> FirestoreResult<()> {
        let _sess: WsSession = db
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
