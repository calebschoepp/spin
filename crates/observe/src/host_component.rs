use std::future::Future;

use anyhow::Result;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::wasmtime::component::Resource;
use spin_core::{async_trait, HostComponent};
use spin_world::v2::observe;
use spin_world::v2::observe::Span as WitSpan;
use tracing::span::EnteredSpan;

use crate::future::ActiveSpans;

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
            span_resources: table::Table::new(1024),
            active_spans: Default::default(),
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
    span_resources: table::Table<Span>,
    pub(crate) active_spans: ActiveSpans,
}

#[async_trait]
impl observe::Host for ObserveData {}

#[async_trait]
impl observe::HostSpan for ObserveData {
    async fn enter(&mut self, name: String) -> Result<Resource<WitSpan>> {
        println!("ENTER\n\n");
        let span = tracing::info_span!("lame_name", "otel.name" = name);
        span.with_subscriber(|(id, dispatch)| {
            dispatch.enter(id);
        });

        let resource_id = self
            .span_resources
            .push(Span { name, inner: span })
            .unwrap();
        Ok(Resource::new_own(resource_id))
    }

    async fn close(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        println!("CLOSE\n\n");
        // Actually close the otel span
        if let Some(thingy) = self.span_resources.get(resource.rep()) {
            println!("Actually closing something");
            thingy.inner.with_subscriber(|(id, dispatch)| {
                dispatch.exit(id);
            });
        }

        Ok(())
    }

    fn drop(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        // TODO: Make sure span ended
        self.span_resources.remove(resource.rep()).unwrap();
        Ok(())
    }
}

struct Span {
    name: String,
    inner: tracing::Span,
}
