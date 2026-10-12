//! In-memory subscriber for checking accidental credential disclosure.

use std::sync::{Arc, Mutex};
use tracing::{
    Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

/// Captures all fields from enabled spans and events without printing them.
#[derive(Clone, Default)]
pub struct RecordingSubscriber(Arc<Mutex<String>>);

impl RecordingSubscriber {
    /// Returns captured fields for assertions in a test fixture.
    pub fn snapshot(&self) -> String {
        self.0.lock().unwrap().clone()
    }
}

impl Visit for RecordingSubscriber {
    fn record_debug(&mut self, _field: &Field, value: &dyn std::fmt::Debug) {
        self.0.lock().unwrap().push_str(&format!("{value:?}"));
    }
}

impl Subscriber for RecordingSubscriber {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        attrs.record(&mut self.clone());
        Id::from_u64(1)
    }
    fn record(&self, _: &Id, record: &Record<'_>) {
        record.record(&mut self.clone());
    }
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        event.record(&mut self.clone());
    }
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}
