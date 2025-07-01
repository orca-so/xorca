use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_staking_pool_address(stake_token_mint: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"staking_pool", stake_token_mint.as_ref()];
    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_get_staking_pool_address() {
        let stake_token_mint = pubkey!("62dSkn5ktwY1PoKPNMArZA4bZsvyemuknWUnnQ2ATTuN");
        let (address, _) = get_staking_pool_address(&stake_token_mint).unwrap();
        let expected_staking_pool_address = pubkey!("kNPV3hhKtqL6NQXGZTj4GpnXzVaoHxHgzkxNTUCQSAo");
        assert_eq!(address, expected_staking_pool_address);
    }
}
