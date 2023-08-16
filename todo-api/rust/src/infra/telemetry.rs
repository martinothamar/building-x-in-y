use anyhow::Result;
use axum::extract::MatchedPath;
use axum::http::Request;
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::TraceLayer;
use tracing::{info, info_span, Span};
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

// fn pkg_name() -> String {
//     std::env::var("CARGO_PKG_NAME").unwrap()
// }

type TraceMiddleware<B> = TraceLayer<SharedClassifier<ServerErrorsAsFailures>, fn(&Request<B>) -> Span>;

pub(crate) fn tracing_middleware<B>() -> TraceMiddleware<B> {
    TraceLayer::new_for_http().make_span_with(make_span)
}

fn make_span<B>(request: &Request<B>) -> Span {
    // Log the matched route's path (with placeholders not filled in).
    // Use request.uri() or OriginalUri if you want the real path.
    let matched_path = request.extensions().get::<MatchedPath>().map(MatchedPath::as_str);

    info_span!(
        "http_request",
        method = ?request.method(),
        matched_path,
    )
}
