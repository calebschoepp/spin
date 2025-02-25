mod host;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::bail;
use indexmap::IndexSet;
use opentelemetry::{
    trace::{SpanContext, SpanId, TraceContextExt},
    Context,
};
use opentelemetry_sdk::{
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    runtime::Tokio,
    trace::{BatchSpanProcessor, SpanProcessor},
    Resource,
};
use spin_factors::{Factor, PrepareContext, RuntimeFactors, SelfInstanceBuilder};
use spin_telemetry::{detector::SpinResourceDetector, env::OtlpProtocol};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct OtelFactor {
    processor: Arc<BatchSpanProcessor<Tokio>>,
}

impl Factor for OtelFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::wasi::otel::tracing::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: spin_factors::RuntimeFactors>(
        &self,
        _ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: spin_factors::RuntimeFactors>(
        &self,
        _: spin_factors::PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        Ok(InstanceState {
            state: Arc::new(RwLock::new(State {
                guest_span_contexts: Default::default(),
                active_spans: Default::default(),
                original_host_span_id: None,
            })),
            processor: self.processor.clone(),
        })
    }
}

impl OtelFactor {
    pub fn new() -> anyhow::Result<Self> {
        // TODO: Configuring the processor should move to init
        // This will configure the exporter based on the OTEL_EXPORTER_* environment variables.
        let exporter = match OtlpProtocol::traces_protocol_from_env() {
            OtlpProtocol::Grpc => opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .build()?,
            OtlpProtocol::HttpProtobuf => opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .build()?,
            OtlpProtocol::HttpJson => bail!("http/json OTLP protocol is not supported"),
        };
        let mut processor = opentelemetry_sdk::trace::BatchSpanProcessor::builder(
            exporter,
            opentelemetry_sdk::runtime::Tokio,
        )
        .build();
        // This is a hack b/c we know the version of this crate will be the same as the version of Spin
        let spin_version = env!("CARGO_PKG_VERSION").to_string();
        processor.set_resource(&Resource::from_detectors(
            Duration::from_secs(5),
            vec![
                // Set service.name from env OTEL_SERVICE_NAME > env OTEL_RESOURCE_ATTRIBUTES > spin
                // Set service.version from Spin metadata
                Box::new(SpinResourceDetector::new(spin_version)),
                // Sets fields from env OTEL_RESOURCE_ATTRIBUTES
                Box::new(EnvResourceDetector::new()),
                // Sets telemetry.sdk{name, language, version}
                Box::new(TelemetryResourceDetector),
            ],
        ));
        Ok(Self {
            processor: Arc::new(processor),
        })
    }
}

pub struct InstanceState {
    pub(crate) state: Arc<RwLock<State>>,
    pub(crate) processor: Arc<BatchSpanProcessor<Tokio>>,
}

impl SelfInstanceBuilder for InstanceState {}

/// Internal state of the OtelFactor instance state.
///
/// This data lives here rather than directly on InstanceState so that we can have multiple things
/// take Arc references to it.
pub(crate) struct State {
    /// A mapping between immutable [SpanId]s and the actual [BoxedSpan] created by our tracer.
    // TODO: Rename to not include "guest"
    // TODO: Merge with active_spans
    pub(crate) guest_span_contexts: HashMap<SpanId, SpanContext>,

    /// A stack of [SpanIds] for all the active spans. The topmost span is the active span.
    ///
    /// When a span is ended it is removed from this stack (regardless of whether is the
    /// active span) and all other spans are shifted back to retain relative order.
    pub(crate) active_spans: IndexSet<SpanId>,

    /// Id of the last span emitted from within the host before entering the guest.
    ///
    /// We use this to avoid accidentally reparenting the original host span as a child of a guest
    /// span.
    pub(crate) original_host_span_id: Option<SpanId>,
}

// /// The WIT resource Span. Effectively wraps an [opentelemetry::global::BoxedSpan].
// pub struct GuestSpan {
//     /// The [opentelemetry::global::BoxedSpan] we use to do the actual tracing work.
//     pub inner: BoxedSpan,
// }

/// Manages access to the OtelFactor state for the purpose of maintaining proper span
/// parent/child relationships when WASI Otel spans are being created.
pub struct OtelContext {
    pub(crate) state: Option<Arc<RwLock<State>>>,
}

impl OtelContext {
    /// Creates an [`OtelContext`] from a [`PrepareContext`].
    ///
    /// If [`RuntimeFactors`] does not contain an [`OtelFactor`], then calling
    /// [`OtelContext::reparent_tracing_span`] will be a no-op.
    pub fn from_prepare_context<T: RuntimeFactors, F: Factor>(
        prepare_context: &mut PrepareContext<T, F>,
    ) -> anyhow::Result<Self> {
        let state = match prepare_context.instance_builder::<OtelFactor>() {
            Ok(instance_state) => Some(instance_state.state.clone()),
            Err(spin_factors::Error::NoSuchFactor(_)) => None,
            Err(e) => return Err(e.into()),
        };
        Ok(Self { state })
    }

    /// Reparents the current [tracing] span to be a child of the last active guest span.
    ///
    /// The otel factor enables guests to emit spans that should be part of the same trace as the
    /// host is producing for a request. Below is an example trace. A request is made to an app, a
    /// guest span is created and then the host is re-entered to fetch a key value.
    ///
    /// ```text
    /// | GET /... _________________________________|
    ///    | execute_wasm_component foo ___________|
    ///       | my_guest_span ___________________|
    ///          | spin_key_value.get |
    /// ```
    ///
    ///  Setting the guest spans parent as the host is trivially done. However, the more difficult
    /// task is having the host factor spans be children of the guest span.
    /// [`OtelContext::reparent_tracing_span`] handles this by reparenting the current span to be
    /// a child of the last active guest span (which is tracked internally in the otel factor).
    ///
    /// Note that if the otel factor is not in your [`RuntimeFactors`] than this is effectively a
    /// no-op.
    ///
    /// This MUST only be called from a factor host implementation function that is instrumented.
    ///
    /// This MUST be called at the very start of the function before any awaits.
    pub fn reparent_tracing_span(&self) {
        // If state is None then we want to return early b/c the factor doesn't depend on the
        // Otel factor and therefore there is nothing to do
        let state = if let Some(state) = self.state.as_ref() {
            state.read().unwrap()
        } else {
            return;
        };

        // If there are no active guest spans then there is nothing to do
        let Some(active_span) = state.active_spans.last() else {
            return;
        };

        // Ensure that we are not reparenting the original host span
        if let Some(original_host_span_id) = state.original_host_span_id {
            if tracing::Span::current()
                .context()
                .span()
                .span_context()
                .span_id()
                .eq(&original_host_span_id)
            {
                panic!("Incorrectly attempting to reparent the original host span. Likely `reparent_tracing_span` was called in an incorrect location.")
            }
        }

        // Now reparent the current span to the last active guest span
        let span_context = state.guest_span_contexts.get(active_span).unwrap().clone();
        let parent_context = Context::new().with_remote_span_context(span_context);
        tracing::Span::current().set_parent(parent_context);
    }
}
