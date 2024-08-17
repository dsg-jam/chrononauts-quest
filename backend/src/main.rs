use anyhow::Context;
use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("0.0.0.0:8080").await?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) -> anyhow::Result<()> {
    let addr = stream.peer_addr().context("missing peer addr")?;

    let ws_stream = tokio_tungstenite::accept_async(stream).await?;

    let (write, read) = ws_stream.split();
    todo!()
}
