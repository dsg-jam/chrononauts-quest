use std::net::IpAddr;
use std::str::FromStr;

use anyhow::Context;
use futures::TryFutureExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tracing::Instrument;

use self::state::State;

mod board;
mod consts;
mod logging;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    tracing::info!(version = consts::VERSION);

    let state = State::new().await?;

    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8080);
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(
            serve_connection(state.clone(), stream)
                .inspect_err(|err| tracing::error!(err = &**err, "connection error"))
                .instrument(tracing::info_span!(
                    "connection",
                    session = tracing::field::Empty
                )),
        );
    }

    Ok(())
}

async fn serve_connection(state: State, stream: TcpStream) -> anyhow::Result<()> {
    let peer = stream.peer_addr().context("missing peer addr")?;
    let span = tracing::Span::current();

    tracing::trace!("accepting websocket connection");
    let mut ws_headers = None;
    let mut ws_stream = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, response| {
        ws_headers = Some(WsHeaders::extract(req));
        Ok(response)
    })
    .await?;

    let ws_headers = match ws_headers {
        Some(Ok(v)) => v,
        other => {
            if let Some(Err(err)) = other {
                tracing::warn!(err = &*err, "dropping connection due to invalid headers");
            }
            let _ = ws_stream
                .close(Some(CloseFrame {
                    code: CloseCode::Invalid,
                    reason: "Invalid Headers".into(),
                }))
                .await;
            return Ok(());
        }
    };

    let session_id = state
        .start_ws_session(ws_headers.client_ip.unwrap_or_else(|| peer.ip()))
        .await?;
    span.record("session", &session_id);

    if let Err(err) = board::serve(state.clone(), ws_stream).await {
        tracing::error!(err = &*err, "error while serving board");
    }

    if let Err(err) = state.end_ws_session(&session_id).await {
        tracing::warn!(err = &*err, "failed to store end of ws session");
    }

    Ok(())
}

#[derive(Default)]
struct WsHeaders {
    client_ip: Option<IpAddr>,
}

impl WsHeaders {
    fn extract(req: &Request) -> anyhow::Result<Self> {
        let mut this = Self::default();
        this.update_from_request(req)?;
        Ok(this)
    }

    fn update_from_request(&mut self, req: &Request) -> anyhow::Result<()> {
        let headers = req.headers();
        // See: <https://cloud.google.com/load-balancing/docs/https/#x-forwarded-for_header>
        if let Some(value) = headers.get("X-Forwarded-For") {
            tracing::trace!(?value, "parsing X-Forwarded-For header");
            let mut ips = value.to_str()?.split(',').rev().map(IpAddr::from_str);
            let _load_balancer_ip = ips
                .next()
                .context("missing load-balancer-ip in X-Forwarded-For")??;
            let client_ip = ips
                .next()
                .context("missing client-ip in X-Forwarded-For")??;
            self.client_ip = Some(client_ip);
        }
        Ok(())
    }
}
