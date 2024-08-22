use std::io::Write;
use std::pin::pin;

use backend_api::labyrinth::{Action, Direction};
use backend_api::{BoardMessage, DeviceId};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

// don't want to hardcode the password here and also don't wanna turn the backend into a library.
#[allow(unused)]
#[path = "../src/consts.rs"]
mod consts;

async fn connect() -> anyhow::Result<WebSocketStream<impl AsyncRead + AsyncWrite + Unpin>> {
    let url = std::env::args().nth(1);
    let url = url.as_deref().unwrap_or("wss://api.chrononauts.quest");
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        anyhow::bail!("url must start with ws:// or wss://");
    }
    let url = format!("{url}/board?password={}", consts::BOARD_PASSWORD);

    println!("connecting to: {url}");
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    Ok(ws_stream)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ws_stream = connect().await?;
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

    let mut device = DeviceId::Player1;
    let mut input_lines = stream_input_lines();
    print_prompt(true);
    loop {
        let event = tokio::select! {
            msg = read.next() => msg.map(Event::Message).unwrap_or(Event::Stop),
            line = input_lines.next() => line.map(Event::InputLine).unwrap_or(Event::Stop),
        };

        let mut show_help = false;
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
                "frequency_tuned" | "ft" => {
                    write.send(BoardMessage::FrequencyTuned).await.unwrap();
                }
                "set_device player1" => {
                    device = DeviceId::Player1;
                }
                "set_device player2" => {
                    device = DeviceId::Player2;
                }
                _ => {
                    if let Some(action) = parse_labyrinth_action(device, line.trim()) {
                        write
                            .send(BoardMessage::LabyrinthAction(action))
                            .await
                            .unwrap();
                    } else {
                        println!("unrecognized command");
                        show_help = true;
                    }
                }
            },
        }
        print_prompt(show_help);
    }

    Ok(())
}

fn parse_labyrinth_action(device: DeviceId, cmd: &str) -> Option<Action> {
    let long_form_cmd = match cmd {
        "lu" => "look_up",
        "ld" => "look_down",
        "ll" => "look_left",
        "lr" => "look_right",
        "su" => "step_up",
        "sd" => "step_down",
        "sl" => "step_left",
        "sr" => "step_right",
        other => other,
    };
    let (action, direction) = long_form_cmd.split_once('_')?;
    let step = match action {
        "step" => true,
        "look" => false,
        _ => return None,
    };
    let direction = match direction {
        "up" => Direction::Up,
        "down" => Direction::Down,
        "left" => Direction::Left,
        "right" => Direction::Right,
        _ => return None,
    };
    Some(Action {
        device,
        direction,
        step,
    })
}

fn print_prompt(show_help: bool) {
    if show_help {
        println!("Available commands:");
        println!("  frequency_tuned|ft");
        println!();
        println!("Labyrinth:");
        println!("  set_device <device>");
        println!("  lu|look_up");
        println!("  ld|look_down");
        println!("  ll|look_left");
        println!("  lr|look_right");
        println!("  su|step_up");
        println!("  sd|step_down");
        println!("  sl|step_left");
        println!("  sr|step_right");
        println!();
    }
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
