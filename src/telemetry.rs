//! OpenTelemetry traces for agent runs.
//!
//! Spans are created with the `tracing` crate (`agent.run`, `agent.step`,
//! `agent.state`, `tool.call`) and exported through `tracing-opentelemetry`.
//!
//! * `OTEL_TRACES_FILE` writes a JSON document when the process shuts down.
//!   No collector is required. This is how `trace-demo` produces a sample.
//! * `OTEL_EXPORTER_OTLP_ENDPOINT` (for example `http://localhost:4318`) also
//!   installs an OTLP/HTTP exporter aimed at a collector or Jaeger.

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::{SdkTracerProvider, SpanData, SpanExporter};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct TelemetryGuard {
    provider: SdkTracerProvider,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Err(err) = self.provider.force_flush() {
            eprintln!("trace flush failed: {err}");
        }
        if let Err(err) = self.provider.shutdown()
            && !err.to_string().contains("AlreadyShutdown")
        {
            eprintln!("trace shutdown failed: {err}");
        }
    }
}

pub fn init_from_env(traces_file_override: Option<PathBuf>) -> TelemetryGuard {
    let traces_file = traces_file_override.or_else(|| {
        std::env::var("OTEL_TRACES_FILE")
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    });
    let otlp = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|value| !value.is_empty())
        .is_some()
        || std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
            .ok()
            .filter(|value| !value.is_empty())
            .is_some();
    init(traces_file, otlp)
}

pub fn init(traces_file: Option<PathBuf>, otlp: bool) -> TelemetryGuard {
    let mut builder = SdkTracerProvider::builder();
    if let Some(path) = traces_file {
        builder = builder.with_simple_exporter(JsonFileSpanExporter::new(path));
    }
    if otlp {
        match opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .build()
        {
            Ok(exporter) => {
                builder = builder.with_batch_exporter(exporter);
            }
            Err(err) => eprintln!("OTLP exporter disabled: {err}"),
        }
    }
    let provider = builder.build();
    let tracer = provider.tracer("actix_autogpt");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    if tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .try_init()
        .is_err()
    {
        eprintln!(
            "tracing subscriber was already initialized; spans follow the existing subscriber"
        );
    }

    TelemetryGuard { provider }
}

#[derive(Debug)]
struct JsonFileSpanExporter {
    path: PathBuf,
    spans: Mutex<Vec<serde_json::Value>>,
}

impl JsonFileSpanExporter {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            spans: Mutex::new(Vec::new()),
        }
    }
}

impl SpanExporter for JsonFileSpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let mut guard = self
            .spans
            .lock()
            .map_err(|err| OTelSdkError::InternalFailure(format!("trace file lock: {err}")))?;
        for span in batch {
            guard.push(span_to_json(&span));
        }
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: std::time::Duration) -> OTelSdkResult {
        let guard = self
            .spans
            .lock()
            .map_err(|err| OTelSdkError::InternalFailure(format!("trace file lock: {err}")))?;
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|err| OTelSdkError::InternalFailure(format!("create trace dir: {err}")))?;
        }
        let mut file = File::create(&self.path).map_err(|err| {
            OTelSdkError::InternalFailure(format!("create {}: {err}", self.path.display()))
        })?;
        let body = serde_json::to_vec_pretty(&*guard)
            .map_err(|err| OTelSdkError::InternalFailure(format!("encode traces: {err}")))?;
        file.write_all(&body)
            .map_err(|err| OTelSdkError::InternalFailure(format!("write traces: {err}")))?;
        file.write_all(b"\n")
            .map_err(|err| OTelSdkError::InternalFailure(format!("write traces: {err}")))?;
        Ok(())
    }
}

fn span_to_json(span: &SpanData) -> serde_json::Value {
    let attributes: Vec<serde_json::Value> = span
        .attributes
        .iter()
        .map(|attribute| {
            serde_json::json!({
                "key": attribute.key.as_str(),
                "value": format!("{:?}", attribute.value),
            })
        })
        .collect();
    serde_json::json!({
        "name": span.name.as_ref(),
        "trace_id": span.span_context.trace_id().to_string(),
        "span_id": span.span_context.span_id().to_string(),
        "parent_span_id": span.parent_span_id.to_string(),
        "kind": format!("{:?}", span.span_kind),
        "start_time": system_time_micros(span.start_time),
        "end_time": system_time_micros(span.end_time),
        "status": format!("{:?}", span.status),
        "attributes": attributes,
    })
}

fn system_time_micros(time: SystemTime) -> u128 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tracing_subscriber::Registry;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn tool_call_span_is_written_to_the_trace_file() {
        let path = std::env::temp_dir().join(format!(
            "autogpt-spans-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_file(&path);
        let exporter = JsonFileSpanExporter::new(path.clone());
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let tracer = provider.tracer("test");
        let layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let parent = tracing::info_span!("agent.step", agent = "Backend Developer");
            let _guard = parent.enter();
            let child = tracing::info_span!(
                "tool.call",
                tool = "cargo_build",
                agent = "Backend Developer"
            );
            let _entered = child.enter();
        });

        provider.force_flush().expect("flush spans");
        provider.shutdown().expect("shutdown tracer");
        let body = fs::read_to_string(&path).expect("trace file");
        assert!(body.contains("tool.call"), "{body}");
        assert!(body.contains("agent.step"), "{body}");
        let _ = fs::remove_file(&path);
    }
}
