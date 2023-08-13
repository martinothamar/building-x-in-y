use std::rc::Rc;

use anyhow::Result;

use opentelemetry::{
    global::{self, BoxedTracer},
    sdk::trace::TracerProvider
};

pub(crate) fn init() -> Result<()> {

    let provider = TracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build();

    global::set_tracer_provider(provider);

    Ok(())
}

pub(crate) fn shutdown() {
    global::shutdown_tracer_provider();
}

fn tracer_name() -> String {
    std::env::var("CARGO_PKG_NAME").unwrap()
}

pub(crate) fn tracer() ->  Rc<BoxedTracer> {
    thread_local! {
        static TRACER: Rc<BoxedTracer> = Rc::new(global::tracer(tracer_name()));
    }

    TRACER.with(Rc::clone)
}
