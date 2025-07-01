use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_lst(stake_token_mint_account: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"lst_token_mint", stake_token_mint_account.as_ref()];
    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_get_lst() {
        let stake_token_mint_account = pubkey!("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE");
        let (address, _) = get_lst(&stake_token_mint_account).unwrap();
        let expected_pending_claim_address: Pubkey =
            pubkey!("8TTq4pnkPqfzcSyQt7teMJZmk7gATbM2mUv3HPyRXYbL");
        assert_eq!(address, expected_pending_claim_address);
    }
}
