use crate::{
    cpi::system::{CreateAccount, SYSTEM_PROGRAM_ID},
    error::ErrorCode,
    state::ProgramAccount,
};
use pinocchio::{
    account_info::{AccountInfo, RefMut},
    instruction::Signer,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

pub fn get_account_info(
    accounts: &[AccountInfo],
    index: usize,
) -> Result<&AccountInfo, ProgramError> {
    if accounts.len() <= index {
        return Err(ErrorCode::NotEnoughAccountKeys.into());
    }
    Ok(&accounts[index])
}

/// This function does not do any assertions on the account owner or role.
/// It is the responsibility of the caller to ensure that the account is owned by the correct program.
pub fn create_account(
    system_program: &AccountInfo,
    funder: &AccountInfo,
    new_account: &AccountInfo,
    space: usize,
    owner: &Pubkey,
    signers: &[Signer],
) -> ProgramResult {
    if new_account.is_owned_by(&SYSTEM_PROGRAM_ID) {
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(space);

        CreateAccount {
            program: system_program,
            from: funder,
            to: new_account,
            lamports,
            space: space as u64,
            owner,
        }
        .invoke_signed(signers)?;
    }

    Ok(())
}

pub fn create_program_account<'a, T: ProgramAccount>(
    system_program: &AccountInfo,
    funder: &AccountInfo,
    new_account: &'a AccountInfo,
    signers: &[Signer],
) -> Result<RefMut<'a, T>, ProgramError> {
    create_account(
        system_program,
        funder,
        new_account,
        T::LEN,
        &crate::ID,
        signers,
    )?;
    let mut data = new_account.try_borrow_mut_data()?;
    data[0] = T::DISCRIMINATOR as u8;
    Ok(T::from_bytes_mut(data))
}
