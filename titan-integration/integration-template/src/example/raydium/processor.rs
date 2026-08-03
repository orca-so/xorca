use solana_pubkey::Pubkey;

use super::error::AmmError;

pub const AUTHORITY_AMM: &'static [u8] = b"amm authority";

pub fn authority_id(program_id: &Pubkey, amm_seed: &[u8], nonce: u8) -> Result<Pubkey, AmmError> {
    Pubkey::create_program_address(&[amm_seed, &[nonce]], program_id)
        .map_err(|_| AmmError::InvalidProgramAddress.into())
}
