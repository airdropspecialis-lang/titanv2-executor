use arc_swap::ArcSwap;
use solana_sdk::hash::Hash;
use std::sync::Arc;

pub struct BlockhashCache {
    current: ArcSwap<Hash>,
}

impl BlockhashCache {
    pub fn new(initial: Hash) -> Self {
        Self {
            current: ArcSwap::from_pointee(initial),
        }
    }

    pub fn get(&self) -> Arc<Hash> {
        self.current.load_full()
    }

    pub fn set(&self, h: Hash) {
        self.current.store(Arc::new(h));
    }
}
