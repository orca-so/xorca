use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_pending_withdraw(
    staking_pool: &Pubkey,
    unstaker: &Pubkey,
    withdraw_index: &u8,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[
        b"pending_withdraw",
        staking_pool.as_ref(),
        unstaker.as_ref(),
        &[*withdraw_index],
    ];
    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_get_pending_withdraw() {
        let staking_pool = pubkey!("kNPV3hhKtqL6NQXGZTj4GpnXzVaoHxHgzkxNTUCQSAo");
        let unstaker = pubkey!("A1tYHa3233WKDX5fZuZNmHMUVTSB12sR1RoVeGT8XV85");
        let withdraw_index = 0;
        let (address, _) = get_pending_withdraw(&staking_pool, &unstaker, &withdraw_index).unwrap();
        let expected_pending_claim_address =
            pubkey!("65hQf2HvGdX8aa92McbXjDcdCYt7GHzPxgiJYyx6sUvK");
        assert_eq!(address, expected_pending_claim_address);
        let withdraw_index = 1;
        let (address, _) = get_pending_withdraw(&staking_pool, &unstaker, &withdraw_index).unwrap();
        let expected_pending_claim_address =
            pubkey!("4NPowK3hFWsWP42XE1ULb52UBbPm5BvbghtSsGASRC9Q");
        assert_eq!(address, expected_pending_claim_address);
    }
}
