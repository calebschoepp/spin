use anyhow::Result;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::wasmtime::component::Resource;
use spin_core::{async_trait, HostComponent};
use spin_telemetry::opentelemetry::global::{self, BoxedSpan, BoxedTracer, ObjectSafeSpan};
use spin_telemetry::opentelemetry::trace::TraceContextExt;
use spin_telemetry::opentelemetry::trace::Tracer;
use spin_telemetry::opentelemetry::Context;
use spin_telemetry::tracing_opentelemetry::OpenTelemetrySpanExt;
use spin_world::v2::observe;
use spin_world::v2::observe::Span as WitSpan;
use tracing::span::EnteredSpan;

pub struct ObserveHostComponent {}

impl ObserveHostComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl HostComponent for ObserveHostComponent {
    type Data = ObserveData;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        observe::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        ObserveData {
            spans: table::Table::new(1024),
            root_context: None,
            span_stack: Vec::new(),
        }
    }
}

impl DynamicHostComponent for ObserveHostComponent {
    fn update_data(&self, _data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        Ok(())
    }
}

/// TODO
pub struct ObserveData {
    spans: table::Table<Span>,
    root_context: Option<Context>,
    span_stack: Vec<u32>,
}

#[async_trait]
impl observe::Host for ObserveData {}

#[async_trait]
impl observe::HostSpan for ObserveData {
    async fn enter(&mut self, name: String) -> Result<Resource<WitSpan>> {
        let tracer = global::tracer("wasi-observe");

        if self.root_context.is_none() {
            self.root_context = Some(tracing::Span::current().context());
        }

        let current_context: Context;
        if let Some(span) = self.span_stack.last() {
            current_context = self.spans.get(*span).unwrap().inner.clone();
        } else {
            current_context = self.root_context.as_ref().unwrap().clone();
        }

        let span = tracer.start_with_context(name.clone(), &current_context);

        let new_context = current_context.clone().with_span(span);

        let resource_id = self
            .spans
            .push(Span {
                name,
                inner: new_context,
            })
            .unwrap();

        self.span_stack.push(resource_id);

        Ok(Resource::new_own(resource_id))
    }

    async fn close(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        if let Some(my_span) = self.spans.get_mut(resource.rep()) {
            my_span.inner.span().end();
            self.span_stack.pop();
        }

        // Thoughts: Iterate back through the stack until I find the span that we're closing and
        // close all the spans on the stack that were passed. If we don't find it go boom

        Ok(())
    }

    fn drop(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        // TODO: Make sure span ended
        self.spans.remove(resource.rep()).unwrap();
        Ok(())
    }
}

struct Span {
    name: String,
    inner: Context,
}
