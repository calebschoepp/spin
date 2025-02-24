// use std::time::SystemTime;

use anyhow::anyhow;
use anyhow::Result;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::Context;
use opentelemetry_sdk::trace::SpanProcessor;
use spin_core::async_trait;
use spin_world::wasi::otel::tracing as wasi_otel;
use spin_world::wasi::otel::tracing::SpanContext;
use tracing::span;

use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::InstanceState;

#[async_trait]
impl wasi_otel::Host for InstanceState {
    async fn on_start(
        &mut self,
        span_data: wasi_otel::SpanData,
        _parent: wasi_otel::SpanContext,
    ) -> Result<()> {
        let mut state = self.state.write().unwrap();

        // Before we ever create any new spans make sure we track the original host span ID
        if state.original_host_span_id.is_none() {
            state.original_host_span_id = Some(
                tracing::Span::current()
                    .context()
                    .span()
                    .span_context()
                    .span_id(),
            );
        }

        // // Get span's parent based on whether it's a new root and whether there are any active spans
        // let parent_context = match (false, state.active_spans.is_empty()) {
        //     // Not a new root && Active spans -> Last active guest span is parent
        //     (false, false) => {
        //         let span_context = state
        //             .guest_spans
        //             .get(*state.active_spans.last().unwrap())
        //             .unwrap()
        //             .inner
        //             .span_context()
        //             .clone();
        //         Context::new().with_remote_span_context(span_context)
        //     }
        //     // Not a new root && No active spans -> Current host span is parent
        //     (false, true) => tracing::Span::current().context(),
        //     // New root && n/a -> No parent
        //     (true, _) => Context::new(),
        // };

        // Create the underlying opentelemetry span
        // let builder = self.tracer.span_builder(span_data.name);
        // if let Some(kind) = options.span_kind {
        //     builder = builder.with_kind(kind.into());
        // }
        // if let Some(attributes) = options.attributes {
        //     builder = builder.with_attributes(attributes.into_iter().map(Into::into));
        // }
        // if let Some(links) = options.links {
        //     builder = builder.with_links(links.into_iter().map(Into::into).collect());
        // }
        // if let Some(timestamp) = options.timestamp {
        //     builder = builder.with_start_time(timestamp);
        // }
        // let otel_span = builder.start_with_context(
        //     &self.tracer,
        //     &Context::new().with_remote_span_context(parent.into()),
        // );
        // let span_id = otel_span.span_context().span_id();

        // Put the span in our map and push it on to our stack of active spans
        let span_context =
            std::convert::Into::<opentelemetry::trace::SpanContext>::into(span_data.span_context);
        let span_id = span_context.span_id();
        state.guest_span_contexts.insert(span_id, span_context);
        state.active_spans.insert(span_id);

        Ok(())
    }

    async fn on_end(&mut self, span_data: wasi_otel::SpanData) -> Result<()> {
        let mut state = self.state.write().unwrap();

        let span_id = std::convert::Into::<opentelemetry::trace::SpanContext>::into(
            span_data.span_context.clone(),
        )
        .span_id();
        self.processor.on_end(span_data.into());
        if let Some(_guest_span) = state.guest_span_contexts.get_mut(&span_id) {
            // // TODO: Transfer all the data
            // guest_span.end_with_timestamp(span_data.end_time.into());

            // Remove the span from active_spans
            state.active_spans.shift_remove(&span_id);

            Ok(())
        } else {
            // TODO: This seems to be wrong
            Err(anyhow!("BUG: cannot find resource in table"))
        }
        // Ok(())
    }

    async fn current_span_context(&mut self) -> Result<wasi_otel::SpanContext> {
        Ok(tracing::Span::current()
            .context()
            .span()
            .span_context()
            .clone()
            .into())
    }
}

// TODO: Rename module to otel
