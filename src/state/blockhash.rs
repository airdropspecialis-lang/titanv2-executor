use arc_swap::ArcSwap;
use solana_sdk::hash::Hash;
use std::sync::Arc;

#[derive(Clone)]
pub struct BlockhashCache {
    inner: Arc<ArcSwap<Hash>>,
}

impl BlockhashCache {
    pub fn new(initial: Hash) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(initial)),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Hash {
        *self.inner.load_full()
    }

    #[inline(always)]
    pub fn set(&self, hash: Hash) {
        self.inner.store(Arc::new(hash));
    }
}
