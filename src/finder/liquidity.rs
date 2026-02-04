use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

pub struct LiquidityFinder {
    last_slot: u64,
    seen_accounts: HashSet<Pubkey>,
}

impl LiquidityFinder {
    pub fn new() -> Self {
        Self {
            last_slot: 0,
            seen_accounts: HashSet::new(),
        }
    }

    pub fn should_process(&mut self, slot: u64, account: Pubkey) -> bool {
        if self.last_slot == slot && self.seen_accounts.contains(&account) {
            return false;
        }

        if self.last_slot != slot {
            self.seen_accounts.clear();
            self.last_slot = slot;
        }

        self.seen_accounts.insert(account);
        true
    }
}
