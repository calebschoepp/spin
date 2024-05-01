use anyhow::{Context, Result};
use std::{
    future::Future,
    sync::{Arc, RwLock},
};

use spin_core::{Engine, HostComponentDataHandle, Store};

use crate::{host_component::ObserveData, ObserveHostComponent};

pub type ActiveSpans = Arc<RwLock<Vec<()>>>;

struct Instrumented<F> {
    inner: F,
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
        Ok(Instrumented { inner: self })
    }
}

impl<F: Future> Future for Instrumented<F> {
    type Output = F::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

pub struct ObserveContext {
    active_spans: ActiveSpans,
}

impl ObserveContext {
    pub fn new<T>(store: &mut Store<T>, engine: &Engine<T>) -> Result<Self> {
        let handle = engine
            .find_host_component_handle::<ObserveHostComponent>()
            .context("host component handle not found")?;
        let active_spans = store
            .host_components_data()
            .get_or_insert(handle)
            .active_spans
            .clone();
        Ok(Self { active_spans })
    }
}

// TODO: Rename Instrumented and instrument_rename
