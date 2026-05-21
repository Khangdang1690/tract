//! Shared daemon state.

use std::sync::Arc;
use std::time::Instant;

use crate::orchestrator::Orchestrator;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    started_at: Instant,
    orchestrator: Orchestrator,
}

impl AppState {
    pub fn new(orchestrator: Orchestrator) -> Self {
        Self {
            inner: Arc::new(Inner {
                started_at: Instant::now(),
                orchestrator,
            }),
        }
    }

    pub fn started_at(&self) -> Instant {
        self.inner.started_at
    }

    pub fn orchestrator(&self) -> &Orchestrator {
        &self.inner.orchestrator
    }
}
