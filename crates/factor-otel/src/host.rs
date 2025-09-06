use anyhow::anyhow;
use anyhow::Result;
use opentelemetry::trace::TraceContextExt;
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::trace::SpanProcessor;
use spin_world::wasi;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::InstanceState;

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
            .insert(span_context.span_id(), span_context);

        Ok(())
    }

    async fn on_end(&mut self, span_data: wasi::otel::tracing::SpanData) -> Result<()> {
        let mut state = self.state.write().unwrap();

        let span_context: opentelemetry::trace::SpanContext = span_data.span_context.clone().into();
        let span_id: opentelemetry::trace::SpanId = span_context.span_id();

        if state.guest_span_contexts.shift_remove(&span_id).is_none() {
            Err(anyhow!("Trying to end a span that was not started"))?;
        }

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

impl wasi::otel::metrics::Host for InstanceState {
    async fn collect(
        &mut self,
        metrics: wasi::otel::metrics::ResourceMetrics,
    ) -> spin_core::wasmtime::Result<std::result::Result<(), wasi::otel::metrics::OtelError>> {
        let mut rm: opentelemetry_sdk::metrics::data::ResourceMetrics = metrics.into();
        println!("{:?}", rm);
        self.metric_exporter.export(&mut rm).await?;
        Ok(Ok(()))
    }
}
