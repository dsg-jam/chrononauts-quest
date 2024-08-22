use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use anyhow::Context;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tracing::Instrument;

use crate::state::{StateHandle, WsSessionKind};

mod board;
mod website;

type WebSocketStream = tokio_tungstenite::WebSocketStream<TcpStream>;

const SESSION_FIELD: &str = "session";
const KIND_FIELD: &str = "kind";

pub async fn listen(state: StateHandle, port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).await?;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let span = tracing::info_span!(
                    "connection",
                    { SESSION_FIELD } = tracing::field::Empty,
                    { KIND_FIELD } = tracing::field::Empty
                );
                let fut = serve(state.clone(), stream).instrument(span);
                tokio::spawn(fut);
            }
            Err(err) => {
                tracing::error!(
                    err = &err as &dyn std::error::Error,
                    "failed to accept connection"
                );
            }
        }
    }
}

async fn serve(state: StateHandle, stream: TcpStream) {
    let span = tracing::Span::current();

    let Some((mut ws_stream, info)) = accept_connection(stream).await else {
        return;
    };

    // make sure the client provided the correct password
    if !verify_password(&info) {
        tracing::info!("client provided invalid password");
        let _ = ws_stream
            .close(Some(CloseFrame {
                code: CloseCode::Invalid,
                reason: "Invalid password".into(),
            }))
            .await;
        return;
    }

    // track the start of a new session (mostly for telemetry)
    let session_id = match state.start_ws_session(info.kind, info.client_ip).await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!(
                err = &err as &dyn std::error::Error,
                "failed to start ws session"
            );
            let _ = ws_stream
                .close(Some(CloseFrame {
                    code: CloseCode::Error,
                    reason: "Failed to start session".into(),
                }))
                .await;
            return;
        }
    };
    span.record(SESSION_FIELD, &session_id);

    // actually serve the connection
    match info.kind {
        WsSessionKind::Board => {
            if let Err(err) = board::serve(state.clone(), &mut ws_stream).await {
                tracing::error!(err = &*err, "error while serving board");
            }
        }
        WsSessionKind::Website => {
            if let Err(err) = website::serve(state.clone(), &mut ws_stream).await {
                tracing::error!(err = &*err, "error while serving website");
            }
        }
    }

    // close the connection
    let _ = ws_stream.close(None).await;

    // track the end of the session
    if let Err(err) = state.end_ws_session(&session_id).await {
        tracing::warn!(
            err = &err as &dyn std::error::Error,
            "failed to store end of ws session"
        );
    }
}

fn verify_password(info: &ClientInfo) -> bool {
    match info.kind {
        WsSessionKind::Board => info.password == crate::consts::BOARD_PASSWORD,
        WsSessionKind::Website => info.password == crate::consts::WEBSITE_PASSWORD,
    }
}

async fn accept_connection(mut stream: TcpStream) -> Option<(WebSocketStream, ClientInfo)> {
    let Ok(peer) = stream.peer_addr() else {
        let _ = stream.shutdown().await;
        return None;
    };

    tracing::trace!("accepting websocket connection");
    let mut client_info = None;
    let mut ws_stream = {
        let res = tokio_tungstenite::accept_hdr_async(stream, |req: &Request, response| {
            client_info = Some(ClientInfo::from_req(peer, req));
            Ok(response)
        })
        .await;
        match res {
            Ok(stream) => stream,
            Err(err) => {
                tracing::warn!(
                    err = &err as &dyn std::error::Error,
                    "failed to accept websocket connection"
                );
                return None;
            }
        }
    };
    let client_info = match client_info {
        Some(Ok(v)) => v,
        other => {
            if let Some(Err(err)) = other {
                tracing::warn!(
                    err = &*err,
                    "dropping connection due to invalid client info"
                );
            }
            let _ = ws_stream
                .close(Some(CloseFrame {
                    code: CloseCode::Invalid,
                    reason: "Client failed to identify itself".into(),
                }))
                .await;
            return None;
        }
    };

    Some((ws_stream, client_info))
}

struct ClientInfo {
    client_ip: IpAddr,
    kind: WsSessionKind,
    password: String,
}

impl ClientInfo {
    fn from_req(peer: SocketAddr, req: &Request) -> anyhow::Result<Self> {
        let span = tracing::Span::current();

        let uri = req.uri();
        let kind = match uri.path() {
            "/board" => WsSessionKind::Board,
            "/website" => WsSessionKind::Website,
            other => anyhow::bail!("unknown path: {other:?}"),
        };
        // record the connection kind as soon as possible
        span.record(KIND_FIELD, kind.as_str());

        let mut pairs = form_urlencoded::parse(uri.query().unwrap_or("").as_bytes());
        let password = pairs
            .find_map(|(k, v)| (k == "password").then_some(v))
            .ok_or_else(|| anyhow::format_err!("missing 'password' in query"))?
            .into_owned();

        let headers = req.headers();
        // See: <https://cloud.google.com/load-balancing/docs/https/#x-forwarded-for_header>
        let client_ip = if let Some(value) = headers.get("X-Forwarded-For") {
            tracing::trace!(?value, "parsing X-Forwarded-For header");
            let mut ips = value.to_str()?.split(',').map(IpAddr::from_str);
            ips.next()
                .context("missing client-ip in X-Forwarded-For")??
        } else {
            peer.ip()
        };

        Ok(Self {
            client_ip,
            kind,
            password,
        })
    }
}
