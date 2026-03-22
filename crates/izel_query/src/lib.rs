use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// The core query context trait.
/// This allows the compiler to cache and re-use computation results.
pub trait QueryContext {
    fn as_any(&self) -> &dyn Any;
}

/// A simple persistent database for queries.
pub struct Database {
    pub storage: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn get<V: 'static + Send + Sync>(&self, key: &str) -> Option<Arc<V>> {
        self.storage
            .get(key)
            .and_then(|v| v.clone().downcast::<V>().ok())
    }

    pub fn set<V: 'static + Send + Sync>(&mut self, key: String, value: V) {
        self.storage.insert(key, Arc::new(value));
    }
}

impl QueryContext for Database {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
