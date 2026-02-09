use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

/// 🛡️ Titan Conflict Guard
/// Garanton: vetëm 1 attempt për account + slot
pub struct ConflictGuard {
    last_slot: HashMap<Pubkey, u64>,
}

impl ConflictGuard {
    pub fn new() -> Self {
        Self {
            last_slot: HashMap::with_capacity(1024),
        }
    }

    /// Kthen true nëse duhet SKIP
    #[inline(always)]
    pub fn should_skip(&mut self, account: Pubkey, slot: u64) -> bool {
        match self.last_slot.get(&account) {
            Some(&last) if last == slot => true, // 🚫 konflikt
            _ => {
                self.last_slot.insert(account, slot);
                false
            }
        }
    }
}
