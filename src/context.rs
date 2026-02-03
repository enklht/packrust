use rustc_hash::FxHashMap;
use std::any::Any;

use crate::CacheKey;

pub struct Context {
    pub cache: FxHashMap<CacheKey, Box<dyn Any>>,
    pub source: Vec<char>,
    pub lr_stack: Vec<CacheKey>,
    pub call_path: Vec<CacheKey>,
    pub pending_evictions: FxHashMap<CacheKey, Vec<CacheKey>>,
}

impl Context {
    pub fn new(source: impl Into<String>) -> Self {
        Context {
            cache: FxHashMap::default(),
            source: source.into().chars().collect(),
            lr_stack: Vec::new(),
            call_path: Vec::new(),
            pending_evictions: FxHashMap::default(),
        }
    }

    pub(crate) fn schedule_cache_eviction(&mut self, key: CacheKey) {
        let dependents = self.pending_evictions.entry(key).or_default();

        for &ancestor in self.call_path.iter().rev() {
            if ancestor == key {
                break;
            } else {
                dependents.push(ancestor);
            }
        }
    }

    pub(crate) fn execute_cache_eviction(&mut self, key: CacheKey) {
        let Some(dependents) = self.pending_evictions.get(&key).cloned() else {
            return;
        };

        for dependent in dependents {
            self.execute_cache_eviction(dependent);
            self.cache.remove(&dependent);
        }
    }

    pub(crate) fn clear_cache_eviction_schedule(&mut self, key: CacheKey) {
        let Some(dependents) = self.pending_evictions.remove(&key) else {
            return;
        };

        for dependent in dependents {
            self.clear_cache_eviction_schedule(dependent)
        }
    }
}
