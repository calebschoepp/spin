use crate::InstanceState;
use anyhow::anyhow;
use anyhow::Result;
use opentelemetry::trace::SpanContext;
use opentelemetry::trace::TraceContextExt;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::trace::SpanProcessor;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use wasi_otel::wasi;

tokio::task_local! {
    static CURRENT_GUEST_SPAN_CONTEXT: SpanContext;
}
impl wasi::otel::tracing::Host for InstanceState {
    async fn on_start(&mut self, context: wasi::otel::tracing::SpanContext) -> Result<()> {
        let mut state = self.state.write().unwrap();

        // Before we do anything make sure we track the original host span ID for reparenting
        if state.original_host_span_id.is_none() {
            state.original_host_span_id = Some(
                tracing::Span::current()
                    .context()
                    .span()
                    .span_context()
                    .span_id(),
            );
        }

        // Track the guest spans context in our ordered map
        let span_context: opentelemetry::trace::SpanContext = context.into();
        state
            .guest_span_contexts
            .insert(span_context.span_id(), span_context.clone());
        CURRENT_GUEST_SPAN_CONTEXT.sync_scope(span_context, || {});

        Ok(())
    }

    async fn on_end(&mut self, span_data: wasi::otel::tracing::SpanData) -> Result<()> {
        let mut state = self.state.write().unwrap();

        let span_context: opentelemetry::trace::SpanContext = span_data.span_context.clone().into();
        let span_id: opentelemetry::trace::SpanId = span_context.span_id();

        println!("\n\n\n {:?}\n\n\n", CURRENT_GUEST_SPAN_CONTEXT.get());

        if state.guest_span_contexts.shift_remove(&span_id).is_none() {
            Err(anyhow!("Trying to end a span that was not started"))?;
        }

        let Some((_, active_span_context)) = state.guest_span_contexts.last() else {
            panic!("TODO: Should we panic?")
        };
        CURRENT_GUEST_SPAN_CONTEXT.sync_scope(active_span_context.clone(), || {});

        self.span_processor.on_end(span_data.into());

        Ok(())
    }

    async fn outer_span_context(&mut self) -> Result<wasi::otel::tracing::SpanContext> {
        Ok(tracing::Span::current()
            .context()
            .span()
            .span_context()
            .clone()
            .into())
    }
}

// TODO: Create a layer similar to /Users/caleb/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tracing-opentelemetry-0.29.0/src/layer.rs

impl wasi::otel::metrics::Host for InstanceState {
    async fn export(
        &mut self,
        metrics: wasi::otel::metrics::ResourceMetrics,
    ) -> spin_core::wasmtime::Result<std::result::Result<(), wasi::otel::metrics::Error>> {
        let mut rm: opentelemetry_sdk::metrics::data::ResourceMetrics = metrics.into();
        match self.metric_exporter.export(&mut rm).await {
            Ok(_) => Ok(Ok(())),
            Err(e) => match e {
                OTelSdkError::AlreadyShutdown => {
                    let msg = "Shutdown has already been invoked";
                    tracing::error!(msg);
                    Ok(Err(msg.to_string()))
                }
                OTelSdkError::InternalFailure(e) => {
                    let detailed_msg = format!("Internal failure: {}", e);
                    tracing::error!(detailed_msg);
                    Ok(Err("Internal failure.".to_string()))
                }
                OTelSdkError::Timeout(d) => {
                    let detailed_msg = format!("Operation timed out after {} seconds", d.as_secs());
                    tracing::error!(detailed_msg);
                    Ok(Err("Operation timed out.".to_string()))
                }
            },
        }
    }
}
