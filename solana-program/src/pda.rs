use crate::cpi::token::ORCA_MINT_ID;
use pinocchio::pubkey::Pubkey;
use pinocchio_associated_token_account::ID as ATA_PROGRAM_ID;
use pinocchio_token::ID as TOKEN_PROGRAM_ID;

/// Centralized seed definitions for all PDA accounts
pub mod seeds {
    use super::*;
    
    /// State account seeds
    pub fn state_seeds<'a>() -> Vec<&'a [u8]> {
        vec![b"state"]
    }
    
    /// PendingWithdraw account seeds
    pub fn pending_withdraw_seeds<'a>(
        unstaker: &'a Pubkey,
        withdraw_index: &'a [u8],
    ) -> Vec<&'a [u8]> {
        vec![b"pending_withdraw", unstaker.as_ref(), withdraw_index]
    }
    
    /// Vault (ATA) seeds for ORCA token
    pub fn vault_seeds<'a>(state: &'a Pubkey) -> Vec<&'a [u8]> {
        vec![
            state.as_ref(),
            TOKEN_PROGRAM_ID.as_ref(),
            ORCA_MINT_ID.as_ref(),
        ]
    }
}

/// Helper functions for finding PDA addresses
pub mod addresses {
    use super::*;
    use pinocchio::pubkey::find_program_address;

    /// Find the state account address and bump
    pub fn find_state_address() -> (Pubkey, u8) {
        let seeds = seeds::state_seeds();
        find_program_address(&seeds, &crate::ID)
    }

    /// Find the pending withdraw account address and bump
    pub fn find_pending_withdraw_address(unstaker: &Pubkey, withdraw_index: &[u8]) -> (Pubkey, u8) {
        let seeds = seeds::pending_withdraw_seeds(unstaker, withdraw_index);
        find_program_address(&seeds, &crate::ID)
    }

    /// Find the vault (ATA) address and bump for a given state
    pub fn find_vault_address(state: &Pubkey) -> (Pubkey, u8) {
        let seeds = seeds::vault_seeds(state);
        find_program_address(&seeds, &ATA_PROGRAM_ID)
    }
}

/// Re-export commonly used functions for convenience
pub use addresses::*;
pub use seeds::*;
