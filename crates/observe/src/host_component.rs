use anyhow::Result;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::wasmtime::component::Resource;
use spin_core::{async_trait, HostComponent};
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
}

#[async_trait]
impl observe::Host for ObserveData {}

#[async_trait]
impl observe::HostSpan for ObserveData {
    async fn enter(&mut self, name: String) -> Result<Resource<WitSpan>> {
        let span = tracing::info_span!("lame");
        span.with_subscriber(|(id, dispatch)| {
            dispatch.enter(id);
        });

        let resource_id = self.spans.push(Span { name, inner: span }).unwrap();
        Ok(Resource::new_own(resource_id))
    }

    async fn close(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        // Actually close the otel span
        if let Some(thingy) = self.spans.get(resource.rep()) {
            thingy.inner.with_subscriber(|(id, dispatch)| {
                dispatch.exit(id);
            });
        }

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
    inner: tracing::Span,
}
