use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

// Centralized seed definitions (matching the program)
const STATE_SEEDS: &[&[u8]] = &[b"state"];

// Program IDs (same as in the program)
const TOKEN_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const ORCA_MINT_ID: Pubkey = solana_program::pubkey!("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE");

pub fn find_state_address() -> Result<(Pubkey, u8), ProgramError> {
    Pubkey::try_find_program_address(STATE_SEEDS, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

pub fn find_orca_vault_address(state: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[
        state.as_ref(),
        TOKEN_PROGRAM_ID.as_ref(),
        ORCA_MINT_ID.as_ref(),
    ];
    Pubkey::try_find_program_address(seeds, &ATA_PROGRAM_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_find_state_address() {
        let (address, _) = find_state_address().unwrap();
        let state = pubkey!("AaWLLj3o6WTe5GXT2kv9ee4sDBaRQnFX5cM3pcs4gvWQ");
        assert_eq!(address, state);
    }
}
