use anyhow::anyhow;
use anyhow::Result;
use opentelemetry::trace::TraceContextExt;
use opentelemetry_sdk::metrics::reader::MetricReader;
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
    async fn export(
        &mut self,
        metrics: wasi::otel::metrics::ResourceMetrics,
    ) -> Result<Result<(), wasi::otel::metrics::MetricError>, anyhow::Error> {
        use opentelemetry_sdk::metrics::MetricError as M;
        match self.metric_reader.collect(metrics.into()) {
            Ok(_) => (),
            Err(e) => match e {
                M::ExportErr(v) => return Err(anyhow!("Export error: {}", v.to_string())),
                M::Config(v) => return Err(anyhow!("Config error: {}", v)),
                M::InvalidInstrumentConfiguration(v) => {
                    return Err(anyhow!(
                        "Invalid Instrument Configuration error: {}",
                        v.to_string()
                    ))
                }
                M::Other(v) => return Err(anyhow!("Other error: {}", v)),
                _ => panic!("unrecognized error type"),
            },
        };

        self.metric_reader.force_flush(); // TODO: Test whether force_flush is required, or if the PeriodicReader flushes on its own

        Ok(Ok(()))
    }
}
