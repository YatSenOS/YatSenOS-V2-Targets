use super::*;
use alloc::{collections::BTreeMap, sync::Arc};
use spin::RwLock;

#[derive(Debug, Clone)]
pub struct ProcessData {
    pub(super) env: Arc<RwLock<BTreeMap<String, String>>>,
}

impl Default for ProcessData {
    fn default() -> Self {
        Self {
            env: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

impl ProcessData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn env(&self, key: &str) -> Option<String> {
        self.env.read().get(key).cloned()
    }

    pub fn set_env(self, key: &str, val: &str) -> Self {
        self.env.write().insert(key.into(), val.into());
        self
    }
}
