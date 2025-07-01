use crate::generated::programs::XORCA_STAKING_PROGRAM_ID;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub fn get_xorca_staking_pool_address() -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"staking_pool"];

    Pubkey::try_find_program_address(seeds, &XORCA_STAKING_PROGRAM_ID)
        .ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey;

    #[test]
    fn test_get_authority_config_address() {
        let (address, _) = get_xorca_staking_pool_address().unwrap();
        let authority_config = pubkey!("EBaKP1vY2HppkakDjBz42eT1WnZ9U2Gp4aEG94uStt1T");
        assert_eq!(address, authority_config);
    }
}
