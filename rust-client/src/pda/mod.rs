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
    fn test_find_state_address() {
        let (address, _) = find_state_address().unwrap();
        let state = pubkey!("CSqKhyW1cpdyjheAx5HXx4ibcnYrzpL5JywEMAkZixBK");
        assert_eq!(address, state);
    }

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
