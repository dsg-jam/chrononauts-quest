//! Chrononauts backend.
//!
//! # Server
//!
//! Since the server serves both the website and the board, it has two endpoints:
//!
//! - `/website` - Endpoint for website clients.
//! - `/board` - Endpoint for board clients.
//!
//! Using any other path will result in a disconnect.
//!
//! ## "Authentication"
//!
//! Clients must include the 'password' query parameter in their connection request.
//! The passwords can be seen in the [`consts`] module.
//!
//! # Architecture
//!
//! ```text
//!           ┌───────────────┬──────────────────────────────────┐             
//!           │ Backend Crate │                                  │             
//!           ├───────────────┘                                  │             
//!   ┌─────┐ │                               ┌────────────┐     │             
//!   │Board├─┼─┐                          ┌─►│Board Server├───┐ │             
//!   └─────┘ │ │  ┌─────────────────────┐ │  └────────────┘   │ │  ┌─────────┐
//! ┌───────┐ │ ├─►│WebSocket Server     ├─┤  ┌──────────────┐ ├─┼─►│Firestore│
//! │Website├─┼─┘  │api.chrononauts.quest│ └─►│Website Server├─┘ │  └─────────┘
//! └───────┘ │    └─────────────────────┘    └──────────────┘   │             
//!           │                                                  │             
//!           └──────────────────────────────────────────────────┘             
//! ```

use self::state::StateHandle;

mod consts;
mod labyrinth;
mod logging;
mod server;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    tracing::info!(version = consts::VERSION);

    let state = StateHandle::new().await?;

    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(8080);
    server::listen(state, port).await?;
    Ok(())
}
