use std::future::Future;
use std::sync::{Arc, RwLock};

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
            state: Arc::new(RwLock::new(State {
                span_resources: table::Table::new(1024),
                active_spans: Default::default(),
            })),
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
    pub(crate) state: Arc<RwLock<State>>,
}

#[async_trait]
impl observe::Host for ObserveData {}

pub(crate) struct State {
    pub span_resources: table::Table<GuestSpan>,
    pub active_spans: Vec<u32>,
}

impl State {
    /// TODO: Both exits the guest spans and removes them from active_spans in reverse order to index
    pub fn close_back_to(&mut self, index: usize) {
        self.active_spans
            .split_off(index)
            .iter()
            .rev()
            .for_each(|id| {
                if let Some(guest_span) = self.span_resources.get(*id) {
                    guest_span.exit();
                } else {
                    tracing::error!("adsf");
                }
            });
    }
}

#[async_trait]
impl observe::HostSpan for ObserveData {
    async fn enter(&mut self, name: String) -> Result<Resource<WitSpan>> {
        println!("Entering {name:?}");
        let span = tracing::info_span!("lame_name", "otel.name" = name);
        let guest_span = GuestSpan {
            name: name.clone(),
            inner: span,
        };
        guest_span.enter();

        let mut state = self.state.write().unwrap();

        let resource_id = state.span_resources.push(guest_span).unwrap();

        state.active_spans.push(resource_id);

        Ok(Resource::new_own(resource_id))
    }

    async fn close(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        println!("Closing");
        let mut state: std::sync::RwLockWriteGuard<State> = self.state.write().unwrap();

        if let Some(index) = state
            .active_spans
            .iter()
            .rposition(|id| *id == resource.rep())
        {
            state.close_back_to(index);
        } else {
            tracing::error!("did not find span to close")
        }

        Ok(())
    }

    fn drop(&mut self, resource: Resource<WitSpan>) -> Result<()> {
        let mut state: std::sync::RwLockWriteGuard<State> = self.state.write().unwrap();

        if let Some(index) = state
            .active_spans
            .iter()
            .rposition(|id| *id == resource.rep())
        {
            state.close_back_to(index);
        } else {
            tracing::error!("did not find span to close")
        }

        state.span_resources.remove(resource.rep()).unwrap();
        Ok(())
    }
}

pub struct GuestSpan {
    pub name: String,
    pub inner: tracing::Span,
}

// Necessary because of phantom don't send

impl GuestSpan {
    pub fn enter(&self) {
        self.inner.with_subscriber(|(id, dispatch)| {
            dispatch.enter(id);
        });
    }

    pub fn exit(&self) {
        self.inner.with_subscriber(|(id, dispatch)| {
            dispatch.exit(id);
        });
    }
}
