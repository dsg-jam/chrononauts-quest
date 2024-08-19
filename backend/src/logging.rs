use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod gcloud;

pub fn init() {
    #[cfg(debug_assertions)]
    let fmt_layer = fmt::layer();
    #[cfg(not(debug_assertions))]
    let fmt_layer = fmt::layer().event_format(gcloud::Format);
    tracing_subscriber::registry()
        .with(
            Targets::new()
                .with_default(Level::INFO)
                .with_target("backend", Level::TRACE),
        )
        .with(fmt_layer)
        .init();
}
