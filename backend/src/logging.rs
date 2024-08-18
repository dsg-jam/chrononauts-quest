use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init() {
    tracing_subscriber::registry()
        .with(
            Targets::new()
                .with_default(Level::INFO)
                .with_target("backend", Level::TRACE),
        )
        .with(fmt::layer())
        .init();
}
