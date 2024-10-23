use pine_tails::configuration::get_configurations;
use pine_tails::startup::engine::Engine;
use pine_tails::startup::prepare::Kits;
use pine_tails::telemetry::{get_subscriber, init_subscriber, LoggerOutbound};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Load Configurations
    let config = get_configurations().expect("Failed to read configuration");

    // NOTE: this subscriber is not the subscriber in domain, this name is from library tracing
    // Init Logger
    let log_subscriber = get_subscriber(
        "info".into(),
        config.application.logger_format,
        LoggerOutbound::new(std::io::stderr),
    );
    init_subscriber(log_subscriber);

    let kits = Kits::prepare(&config)?;
    Engine::build(config, kits)?.spinup().await
}
