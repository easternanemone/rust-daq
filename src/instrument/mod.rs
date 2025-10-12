//! Instrument trait, registry, and implementations.
use crate::core::Instrument;
use std::collections::HashMap;

pub mod mock;
pub mod scpi;

type InstrumentFactory = Box<dyn Fn() -> Box<dyn Instrument> + Send + Sync>;

/// A registry for available instrument types.
pub struct InstrumentRegistry {
    factories: HashMap<String, InstrumentFactory>,
}

impl InstrumentRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Registers a new instrument type.
    pub fn register<F>(&mut self, id: &str, factory: F)
    where
        F: Fn() -> Box<dyn Instrument> + Send + Sync + 'static,
    {
        self.factories.insert(id.to_string(), Box::new(factory));
    }

    /// Creates an instance of an instrument by its ID.
    pub fn create(&self, id: &str) -> Option<Box<dyn Instrument>> {
        self.factories.get(id).map(|factory| factory())
    }

    /// Lists the IDs of all registered instruments.
    pub fn list(&self) -> impl Iterator<Item = String> + '_ {
        self.factories.keys().cloned()
    }
}
