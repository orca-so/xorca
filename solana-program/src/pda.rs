use pinocchio::{instruction::Seed, pubkey::Pubkey};
use pinocchio_associated_token_account::ID as ATA_PROGRAM_ID;

/// Centralized seed definitions for all PDA accounts
pub mod seeds {
    use super::*;

    /// State account seeds - returns raw byte arrays for derive_address
    pub fn state_seeds_raw<'a>() -> [&'a [u8]; 1] {
        [b"state"]
    }

    /// State account seeds - returns Seeds for invoke_signed
    pub fn state_seeds<'a>() -> Vec<Seed<'a>> {
        state_seeds_raw().into_iter().map(Seed::from).collect()
    }

    /// PendingWithdraw account seeds - returns raw byte arrays
    pub fn pending_withdraw_seeds_raw<'a>(
        unstaker: &'a Pubkey,
        withdraw_index: &'a [u8],
    ) -> [&'a [u8]; 3] {
        [b"pending_withdraw", unstaker.as_ref(), withdraw_index]
    }

    /// PendingWithdraw account seeds - returns Seeds for invoke_signed
    pub fn pending_withdraw_seeds<'a>(
        unstaker: &'a Pubkey,
        withdraw_index: &'a [u8],
    ) -> Vec<Seed<'a>> {
        pending_withdraw_seeds_raw(unstaker, withdraw_index)
            .into_iter()
            .map(Seed::from)
            .collect()
    }

    /// Vault (ATA) seeds - returns raw byte arrays for derive_address
    pub fn vault_seeds_raw<'a>(
        state: &'a Pubkey,
        token_program: &'a Pubkey,
        orca_mint: &'a Pubkey,
    ) -> [&'a [u8]; 3] {
        [state.as_ref(), token_program.as_ref(), orca_mint.as_ref()]
    }

    /// Vault (ATA) seeds - returns Seeds for invoke_signed
    pub fn vault_seeds<'a>(
        state: &'a Pubkey,
        token_program: &'a Pubkey,
        orca_mint: &'a Pubkey,
    ) -> Vec<Seed<'a>> {
        vault_seeds_raw(state, token_program, orca_mint)
            .into_iter()
            .map(Seed::from)
            .collect()
    }
}

/// Helper functions for finding PDA addresses
pub mod addresses {
    use super::*;
    use pinocchio::pubkey::find_program_address;

    /// Find the state account address and bump
    pub fn find_state_address() -> (Pubkey, u8) {
        let seeds = seeds::state_seeds_raw();
        find_program_address(&seeds, &crate::ID)
    }

    /// Find the pending withdraw account address and bump
    pub fn find_pending_withdraw_address(unstaker: &Pubkey, withdraw_index: &[u8]) -> (Pubkey, u8) {
        let seeds = seeds::pending_withdraw_seeds_raw(unstaker, withdraw_index);
        find_program_address(&seeds, &crate::ID)
    }

    /// Find the vault (ATA) address and bump for a given state
    pub fn find_vault_address(
        state: &Pubkey,
        token_program: &Pubkey,
        orca_mint: &Pubkey,
    ) -> (Pubkey, u8) {
        let seeds = seeds::vault_seeds_raw(state, token_program, orca_mint);
        find_program_address(&seeds, &ATA_PROGRAM_ID)
    }
}

/// Re-export commonly used functions for convenience
pub use addresses::*;
pub use seeds::*;
