use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn find_state_address(orca_mint: Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"state", orca_mint.as_ref()];

    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_find_state_address() {
        let (address, _) =
            find_state_address(pubkey!("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE")).unwrap();
        let state = pubkey!("85zkjNZLy5HXuB3kkgmKEgN9TThH4P8M8p9yawKgkZBo");
        assert_eq!(address, state);
    }
}
