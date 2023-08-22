use anyhow::Result;
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{self, TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub(crate) fn init() -> Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_env())
        .install_batch(opentelemetry::runtime::Tokio)?;

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(telemetry_layer)
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()?;

    info!("telemetry initialized");
    Ok(())
}

pub(crate) fn shutdown() {
    global::shutdown_tracer_provider();
}

type TraceMiddleware = TraceLayer<SharedClassifier<ServerErrorsAsFailures>, tower_http::trace::DefaultMakeSpan>;

pub(crate) fn tracing_middleware() -> TraceMiddleware {
    TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO))
        .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR))
}
