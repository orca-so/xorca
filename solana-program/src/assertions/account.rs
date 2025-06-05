use base58::ToBase58;
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::Seed,
    program_error::ProgramError,
    pubkey::{Pubkey, find_program_address},
};
use pinocchio_log::log;

use crate::error::ErrorCode;

pub trait Key {
    fn key(&self) -> &Pubkey;
}

impl Key for Pubkey {
    fn key(&self) -> &Pubkey {
        self
    }
}

impl Key for AccountInfo {
    fn key(&self) -> &Pubkey {
        self.key()
    }
}

pub enum AccountRole {
    Signer,
    Writable,
}

pub fn assert_account_role(account: &AccountInfo, roles: &[AccountRole]) -> ProgramResult {
    for role in roles {
        match (role, account.is_signer(), account.is_writable()) {
            (AccountRole::Signer, false, _) => {
                log!(
                    "Account {} is not a signer",
                    account.key().to_base58().as_str()
                );
                return Err(ErrorCode::InvalidAccountRole.into());
            }
            (AccountRole::Writable, _, false) => {
                log!(
                    "Account {} is not writable",
                    account.key().to_base58().as_str()
                );
                return Err(ErrorCode::InvalidAccountRole.into());
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn assert_account_owner(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if !account.is_owned_by(owner) {
        log!(
            "Account {} is not owned by {}",
            account.key().to_base58().as_str(),
            owner.to_base58().as_str()
        );
        return Err(ErrorCode::IncorrectOwner.into());
    }
    Ok(())
}

pub fn assert_account_seeds(
    account: &AccountInfo,
    program_id: &Pubkey,
    seeds: &[Seed],
) -> Result<[u8; 1], ProgramError> {
    let seed_bytes = seeds
        .iter()
        .map(|seed| seed.as_ref())
        .collect::<Vec<&[u8]>>();
    let (address, bump) = find_program_address(&seed_bytes, program_id);
    if account.key() != &address {
        log!(
            "Account {} has wrong seeds",
            account.key().to_base58().as_str()
        );
        return Err(ErrorCode::InvalidSeeds.into());
    }
    Ok([bump])
}

pub fn assert_account_address(account: &impl Key, address: &Pubkey) -> ProgramResult {
    if account.key() != address {
        log!(
            "Account {} has wrong address",
            account.key().to_base58().as_str()
        );
        return Err(ErrorCode::IncorrectAccountAddress.into());
    }
    Ok(())
}
