use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_pending_claim(
    staking_pool: &Pubkey,
    staker: &Pubkey,
    claim_index: &u8,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[
        b"pending_claim",
        staking_pool.as_ref(),
        staker.as_ref(),
        &[*claim_index],
    ];
    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_get_pending_claim() {
        let staking_pool = pubkey!("kNPV3hhKtqL6NQXGZTj4GpnXzVaoHxHgzkxNTUCQSAo");
        let staker = pubkey!("A1tYHa3233WKDX5fZuZNmHMUVTSB12sR1RoVeGT8XV85");
        let claim_index = 0;
        let (address, _) = get_pending_claim(&staking_pool, &staker, &claim_index).unwrap();
        let expected_pending_claim_address =
            pubkey!("Cp3nDZgDZg6qHxAkdxKcQg2CwoqizQKk8v5yddDcs2cm");
        assert_eq!(address, expected_pending_claim_address);
        let claim_index = 1;
        let (address, _) = get_pending_claim(&staking_pool, &staker, &claim_index).unwrap();
        let expected_pending_claim_address =
            pubkey!("CqxSzQs6Q9HYatRaieofVy3ZZRfGbqgCaELRrzpw5bVC");
        assert_eq!(address, expected_pending_claim_address);
    }
}
