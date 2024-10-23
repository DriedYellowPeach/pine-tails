//! Module `telemetry` is for handling logging and tracing.
//! It provides two funcionalities:
//! - setup up logger format and level.
//! - init the global logger.

use is_terminal::IsTerminal;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    registry::LookupSpan,
    EnvFilter, Registry,
};

// TODO: figure out the difference between these format
#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum LoggerFormat {
    Compact,
    Full,
    Pretty,
    Json,
    Bunyan,
}

pub struct LoggerOutbound<W> {
    make_writer: W,
}

impl<W> LoggerOutbound<W>
where
    W: for<'a> MakeWriter<'a> + 'static,
{
    pub fn new(make_writer: W) -> Self {
        Self { make_writer }
    }

    fn fmt_layer_full<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(self.make_writer)
    }

    fn fmt_layer_pretty<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(self.make_writer)
            .pretty()
    }

    fn fmt_layer_json<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(self.make_writer)
            .json()
    }

    fn fmt_layer_compact<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        tracing_subscriber::fmt::Layer::new()
            .with_ansi(std::io::stderr().is_terminal())
            .with_writer(self.make_writer)
            .compact()
            .without_time()
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
    }

    fn fmt_layer_bunyan<S>(self) -> impl Layer<S>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
    {
        BunyanFormattingLayer::new(
            "zero2prod".into(),
            // Output the formatted spans to stdout.
            self.make_writer,
        )
    }
}

/// Get the subscriber for the logger.
/// - `env_filter` can be: "info", "debug", "trace", "warn", "error".
/// - `format` can be: "info", "debug", "trace", "warn", "error".
/// - `logger_outbound` is where the log will be written to.
pub fn get_subscriber<Sink>(
    env_filter: String,
    format: LoggerFormat,
    output: LoggerOutbound<Sink>,
) -> Box<dyn Subscriber + Send + Sync>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let filter_layer = Registry::default().with(env_filter);

    match format {
        LoggerFormat::Compact => Box::new(filter_layer.with(output.fmt_layer_compact())),
        LoggerFormat::Full => Box::new(filter_layer.with(output.fmt_layer_full())),
        LoggerFormat::Pretty => Box::new(filter_layer.with(output.fmt_layer_pretty())),
        LoggerFormat::Json => Box::new(filter_layer.with(output.fmt_layer_json())),
        LoggerFormat::Bunyan => Box::new(
            filter_layer
                .with(JsonStorageLayer)
                .with(output.fmt_layer_bunyan()),
        ),
    }
}

/// Init the subscriber for the logger.
/// Be sure to setup this to collect logs.
pub fn init_subscriber<S>(subscriber: S)
where
    S: Subscriber + Send + Sync,
{
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
