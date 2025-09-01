use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn find_state_address() -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"state"];

    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

pub fn find_orca_vault_address(
    state: &Pubkey,
    orca_mint: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[state.as_ref(), orca_mint.as_ref()];
    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
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
