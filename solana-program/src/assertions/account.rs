use crate::{
    cpi::token::TokenAccount,
    error::ErrorCode,
    state::{AccountDiscriminator, ProgramAccount},
};
use base58::ToBase58;
use borsh::BorshDeserialize;
use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    instruction::Seed,
    program_error::ProgramError,
    pubkey::{create_program_address, find_program_address, Pubkey},
    ProgramResult,
};
use pinocchio_log::log;
use pinocchio_token::ID as SPL_TOKEN_PROGRAM_ID;

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

pub fn assert_account_seeds_with_bump(
    account: &AccountInfo,
    program_id: &Pubkey,
    seeds: &[Seed],
    bump: u8,
) -> ProgramResult {
    let mut seed_bytes = seeds
        .iter()
        .map(|seed| seed.as_ref())
        .collect::<Vec<&[u8]>>();
    let bump_bytes = [bump];
    seed_bytes.push(&bump_bytes);

    let address =
        create_program_address(&seed_bytes, program_id).map_err(|_| ErrorCode::InvalidSeeds)?;
    if account.key() != &address {
        log!(
            "Account {} has wrong seeds",
            account.key().to_base58().as_str()
        );
        return Err(ErrorCode::InvalidSeeds.into());
    }
    Ok(())
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

pub fn assert_account_data<T: ProgramAccount>(
    account: &AccountInfo,
) -> Result<Ref<'_, T>, ProgramError> {
    assert_account_len(account, T::LEN)?;
    assert_account_discriminator(account, &[T::DISCRIMINATOR])?;

    let data = account.try_borrow_data()?;
    Ok(T::from_bytes(data))
}

pub fn assert_account_data_mut<T: ProgramAccount>(
    account: &AccountInfo,
) -> Result<RefMut<'_, T>, ProgramError> {
    assert_account_len(account, T::LEN)?;
    assert_account_discriminator(account, &[T::DISCRIMINATOR])?;

    let data = account.try_borrow_mut_data()?;
    Ok(T::from_bytes_mut(data))
}

pub fn assert_account_len(account: &AccountInfo, length: usize) -> ProgramResult {
    if account.data_len() < length {
        log!(
            "Account {} is incorrect size. Expected at least {} but got {}",
            account.key().to_base58().as_str(),
            length as u64,
            account.data_len() as u64,
        );
        return Err(ErrorCode::InvalidAccountData.into());
    }
    Ok(())
}

pub fn assert_account_discriminator(
    account: &AccountInfo,
    discriminators: &[AccountDiscriminator],
) -> ProgramResult {
    let data = account.try_borrow_data()?;
    if data.len() < 1 {
        log!(
            "Invalid discriminator for account: {}",
            account.key().to_base58().as_str()
        );
        return Err(ErrorCode::InvalidAccountData.into());
    }

    for discriminator in discriminators {
        if *discriminator as u8 == data[0] {
            return Ok(());
        }
    }

    log!(
        "Invalid discriminator for account: {}",
        account.key().to_base58().as_str()
    );
    Err(ErrorCode::InvalidAccountData.into())
}

pub fn assert_external_account_data<T: BorshDeserialize>(
    account: &AccountInfo,
) -> Result<T, ProgramError> {
    let data = account.try_borrow_data()?;
    let account = T::deserialize(&mut &*data).map_err(|_| ErrorCode::InvalidAccountData)?;
    Ok(account)
}

pub fn make_owner_token_account_assertions<'a>(
    owner_token_account: &'a AccountInfo,
    owner_account: &AccountInfo,
    token_mint_account: &AccountInfo,
) -> Result<TokenAccount, ProgramError> {
    assert_account_role(owner_token_account, &[AccountRole::Writable])?;
    assert_account_owner(owner_token_account, &SPL_TOKEN_PROGRAM_ID)?;
    let owner_token_account_data =
        assert_external_account_data::<TokenAccount>(owner_token_account)?;
    if owner_token_account_data.owner != *owner_account.key() {
        log!(
            "Token account data owner {} does not match owner account {} for token mint account {}.",
            owner_token_account_data.owner.to_base58().as_str(),
            owner_account.key().to_base58().as_str(),
            token_mint_account.key().to_base58().as_str()
        );
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if owner_token_account_data.mint != *token_mint_account.key() {
        log!(
            "Token account data mint {} does not match token mint account {}.",
            owner_token_account_data.mint.to_base58().as_str(),
            token_mint_account.key().to_base58().as_str()
        );
        return Err(ErrorCode::InvalidAccountData.into());
    }
    Ok(owner_token_account_data)
}
