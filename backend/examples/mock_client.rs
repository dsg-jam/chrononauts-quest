use std::io::Write;
use std::pin::pin;

use backend_api::{BoardMessage, DeviceId, Direction, LabyrinthAction};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args().skip(1).next();
    let url = url.as_deref().unwrap_or("wss://api.chrononauts.quest");

    println!("connecting to: {url}");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (write, read) = ws_stream.split();

    let write = write
        .sink_map_err(anyhow::Error::from)
        .with_flat_map(|msg: BoardMessage| {
            futures::stream::once(async move {
                let payload = serde_json::to_vec(&msg)?;
                Ok(Message::binary(payload))
            })
        });
    let mut write = pin!(write);

    let read = read.filter_map(|msg| async move {
        match msg {
            Err(err) => Some(Err(anyhow::Error::from(err))),
            Ok(msg) => {
                if !(msg.is_binary() || msg.is_text()) {
                    return None;
                }
                let msg = serde_json::from_slice::<BoardMessage>(&msg.into_data());
                Some(msg.map_err(anyhow::Error::from))
            }
        }
    });
    let mut read = pin!(read);

    let mut input_lines = stream_input_lines();
    print_prompt();
    loop {
        let event = tokio::select! {
            msg = read.next() => msg.map(Event::Message).unwrap_or(Event::Stop),
            line = input_lines.next() => line.map(Event::InputLine).unwrap_or(Event::Stop),
        };

        match event {
            Event::Stop => {
                println!();
                break;
            }
            Event::Message(Ok(msg)) => {
                println!();
                println!("BACKEND: {msg:?}");
            }
            Event::Message(Err(err)) => {
                println!();
                println!("ERROR: {err}");
            }
            Event::InputLine(line) => match line.trim() {
                "up" => {
                    write
                        .send(BoardMessage::LabyrinthAction(LabyrinthAction {
                            device: DeviceId::Player1,
                            direction: Direction::Up,
                            step: true,
                        }))
                        .await
                        .unwrap();
                }
                _ => {
                    println!("unrecognized command");
                }
            },
        }
        print_prompt();
    }

    Ok(())
}

fn print_prompt() {
    println!("commands: up");
    print!("> ");
    let _ = std::io::stdout().flush();
}

enum Event {
    Stop,
    Message(anyhow::Result<BoardMessage>),
    InputLine(String),
}

fn stream_input_lines() -> ReceiverStream<String> {
    let (tx, rx) = mpsc::channel(1);
    tokio::task::spawn_blocking(move || {
        use std::io::BufRead;
        let mut line = String::new();
        let mut stdin = std::io::stdin().lock();
        while !tx.is_closed() {
            let n = stdin.read_line(&mut line).unwrap();
            if n == 0 {
                break;
            }
            let _ = tx.blocking_send(std::mem::take(&mut line));
        }
    });
    ReceiverStream::new(rx)
}
