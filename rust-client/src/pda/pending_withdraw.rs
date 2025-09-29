use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn find_pending_withdraw_pda(
    unstaker: &Pubkey,
    withdraw_index: &u8,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"pending_withdraw", unstaker.as_ref(), &[*withdraw_index]];
    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_find_pending_withdraw_pda() {
        let unstaker = pubkey!("A1tYHa3233WKDX5fZuZNmHMUVTSB12sR1RoVeGT8XV85");
        let withdraw_index = 0;
        let (address, _) = find_pending_withdraw_pda(&unstaker, &withdraw_index).unwrap();
        let expected_pending_withdraw_address =
            pubkey!("7hA1R5rPjcj6m7G2HcxnQ82aNumKT9FKQ6ALS2yYXeq6");
        assert_eq!(address, expected_pending_withdraw_address);
        let withdraw_index = 1;
        let (address, _) = find_pending_withdraw_pda(&unstaker, &withdraw_index).unwrap();
        let expected_pending_withdraw_address =
            pubkey!("D6i7r2sBNozTvdKuSSE2HdfqPb9sTd7gNd8jsCtR8quW");
        assert_eq!(address, expected_pending_withdraw_address);
    }
}
