use anyhow::Context;
use backend_api::{BoardMessage, GameState};
use futures::{SinkExt, StreamExt, TryFutureExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::Message;
use tracing::Instrument;

use self::state::State;

mod consts;
mod logging;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    tracing::info!(version = consts::VERSION);

    let state = State::new().await?;

    let listener = TcpListener::bind("0.0.0.0:8080").await?;

    while let Ok((stream, _)) = listener.accept().await {
        accept_connection(state.clone(), stream);
    }

    Ok(())
}

fn accept_connection(state: State, stream: TcpStream) {
    async fn inner(state: State, stream: TcpStream) -> anyhow::Result<()> {
        let peer = stream.peer_addr().context("missing peer addr")?;
        let span = tracing::Span::current();

        tracing::debug!(%peer, "accepting websocket connection");

        let ws_stream =
            tokio_tungstenite::accept_hdr_async(stream, |_req: &Request, response| Ok(response))
                .await?;
        let session_id = state.start_ws_session(peer.ip()).await?;
        span.record("session", &session_id);

        let (mut write, mut read) = ws_stream.split();

        let payload = serde_json::to_vec(&BoardMessage::GameState(GameState {
            level: backend_api::Level::L0,
        }))
        .unwrap();
        write.send(Message::binary(payload)).await.unwrap();

        while let Some(msg) = read.next().await {
            tracing::debug!("{msg:?}");
        }

        if let Err(err) = state.end_ws_session(&session_id).await {
            tracing::warn!(err = &*err, "failed to store end of ws session");
        }

        Ok(())
    }

    tokio::spawn(
        inner(state, stream)
            .inspect_err(|err| tracing::error!(err = &**err, "connection error"))
            .instrument(tracing::info_span!(
                "connection",
                session = tracing::field::Empty
            )),
    );
}
