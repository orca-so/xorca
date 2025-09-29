use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

const ATA_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub fn find_state_address() -> Result<(Pubkey, u8), ProgramError> {
    Pubkey::try_find_program_address(&[b"state"], &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

pub fn find_orca_vault_address(
    state: &Pubkey,
    token_program: &Pubkey,
    orca_mint: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[state.as_ref(), token_program.as_ref(), orca_mint.as_ref()];
    Pubkey::try_find_program_address(seeds, &ATA_PROGRAM_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_find_state_address() {
        let (address, _) = find_state_address().unwrap();
        let state = pubkey!("CSqKhyW1cpdyjheAx5HXx4ibcnYrzpL5JywEMAkZixBK");
        assert_eq!(address, state);
    }
}
