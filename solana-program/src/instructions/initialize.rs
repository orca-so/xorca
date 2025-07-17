use crate::{
    assertions::account::{
        assert_account_address, assert_account_owner, assert_account_role, assert_account_seeds,
        assert_external_account_data, AccountRole,
    },
    cpi::token::{TokenMint, ORCA_MINT_ID, XORCA_MINT_ID},
    error::ErrorCode,
    event::Event,
    instructions::INITIAL_UPGRADE_AUTHORITY_ID,
    state::state::State,
    util::account::{create_program_account_borsh, get_account_info},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::ID as SPL_TOKEN_PROGRAM_ID;

pub fn process_instruction(accounts: &[AccountInfo], cool_down_period_s: &i64) -> ProgramResult {
    let payer_account = get_account_info(accounts, 0)?;
    let state_account = get_account_info(accounts, 1)?;
    let xorca_mint_account = get_account_info(accounts, 2)?;
    let orca_mint_account = get_account_info(accounts, 3)?;
    let update_authority_account = get_account_info(accounts, 4)?;
    let system_program_account = get_account_info(accounts, 5)?;

    // 1. Payer Account Assertions
    assert_account_role(payer_account, &[AccountRole::Signer, AccountRole::Writable])?;

    // 2. xOrca State Account Assertions
    assert_account_role(state_account, &[AccountRole::Writable])?;
    let mut state_seeds = State::seeds();
    let state_bump = assert_account_seeds(state_account, &crate::ID, &state_seeds)?;
    state_seeds.push(Seed::from(&state_bump));

    if state_account.data_len() > 0 {
        return Err(ErrorCode::StateAccountAlreadyInitialized.into());
    }
    assert_account_owner(state_account, &SYSTEM_PROGRAM_ID)?;

    // 3. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    let xorca_mint_account_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;
    assert_account_address(state_account, &xorca_mint_account_data.mint_authority)?;
    if xorca_mint_account_data.supply != 0 {
        return Err(ErrorCode::InvalidAccountData.into());
    }
    assert_account_address(xorca_mint_account, &XORCA_MINT_ID)?;

    // Verify mint authority is this program
    if xorca_mint_account_data.mint_authority_flag != 1 {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // Verify freeze authority is NULL (no freeze authority)
    if xorca_mint_account_data.freeze_authority_flag != 0 {
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if xorca_mint_account_data.freeze_authority != pinocchio::pubkey::Pubkey::default() {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // 4. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_external_account_data::<TokenMint>(orca_mint_account)?;
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;

    // 5. Update Authority Account Assertions
    assert_account_address(update_authority_account, &INITIAL_UPGRADE_AUTHORITY_ID)?;

    // 6. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // Create the State struct
    let mut state_data = State::default();
    state_data.cool_down_period_s = *cool_down_period_s;
    state_data.update_authority = *update_authority_account.key();

    create_program_account_borsh(
        system_program_account,
        payer_account,
        state_account,
        &[state_seeds.as_slice().into()],
        &state_data,
    )?;

    Ok(())
}
