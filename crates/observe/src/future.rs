use anyhow::{Context, Result};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    sync::{Arc, RwLock},
};

use spin_core::{Engine, Store};

use crate::{host_component::State, ObserveHostComponent};

pin_project! {
    struct Instrumented<F> {
        #[pin]
        inner: F,
        observe_context: ObserveContext,
    }

    impl<F> PinnedDrop for Instrumented<F> {
        fn drop(this: Pin<&mut Self>) {
            this.project().observe_context.drop_all();
        }
    }
}

pub trait FutureExt: Future + Sized {
    fn instrument_rename(
        self,
        observe_context: ObserveContext,
    ) -> Result<impl Future<Output = Self::Output>>;
}

impl<F: Future> FutureExt for F {
    fn instrument_rename(
        self,
        observe_context: ObserveContext,
    ) -> Result<impl Future<Output = Self::Output>> {
        Ok(Instrumented {
            inner: self,
            observe_context,
        })
    }
}

impl<F: Future> Future for Instrumented<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        {
            let state = this.observe_context.state.write().unwrap();

            // TODO: Make it a method on state
            for span_id in state.active_spans.iter() {
                println!("Attempting to enter {span_id:?}");
                if let Some(span_resource) = state.span_resources.get(*span_id) {
                    span_resource.enter();
                } else {
                    tracing::error!("No span to enter")
                }
            }
        }

        let ret = this.inner.poll(cx);

        {
            let state = this.observe_context.state.write().unwrap();

            // TODO: Make it a method on state
            for span_id in state.active_spans.iter().rev() {
                println!("Attempting to exit {span_id:?}");

                if let Some(span_resource) = state.span_resources.get(*span_id) {
                    span_resource.exit();
                } else {
                    tracing::error!("span already dropped")
                }
            }
        }

        ret
    }
}

pub struct ObserveContext {
    state: Arc<RwLock<State>>,
}

impl ObserveContext {
    pub fn new<T>(store: &mut Store<T>, engine: &Engine<T>) -> Result<Self> {
        let handle = engine
            .find_host_component_handle::<Arc<ObserveHostComponent>>()
            .context("host component handle not found")?;
        let state = store
            .host_components_data()
            .get_or_insert(handle)
            .state
            .clone();
        Ok(Self { state })
    }

    fn drop_all(&self) {
        let mut state: std::sync::RwLockWriteGuard<State> = self.state.write().unwrap();

        state.close_back_to(0);
    }
}

// TODO: Rename everything

// Problems we need to fix
// - Cancelling future
// - Guest mismanages inner spans
// - Guest mismanages outer span and holds it in global state
