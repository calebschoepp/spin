use anyhow::Result;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::wasmtime::component::Resource;
use spin_core::{async_trait, HostComponent};
use spin_world::v2::tracing;
use spin_world::v2::tracing::Span as WitSpan;

pub struct TracingHostComponent {}

impl TracingHostComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl HostComponent for TracingHostComponent {
    type Data = TracingData;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        tracing::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        TracingData {
            spans: table::Table::new(1024),
        }
    }
}

impl DynamicHostComponent for TracingHostComponent {
    fn update_data(&self, _data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        Ok(())
    }
}

/// TODO
pub struct TracingData {
    spans: table::Table<Span>,
}

#[async_trait]
impl tracing::Host for TracingData {
    async fn span_enter(&mut self, name: String) -> Result<Resource<WitSpan>> {
        let resource_id = self.spans.push(Span { name }).unwrap();
        Ok(Resource::new_own(resource_id))
    }
}

#[async_trait]
impl tracing::HostSpan for TracingData {
    async fn close(&mut self, _resource: Resource<WitSpan>) -> Result<()> {
        // Actually close the otel span
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
}
