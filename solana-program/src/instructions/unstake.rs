use crate::{
    assertions::account::{
        assert_account_address, assert_account_data, assert_account_data_mut, assert_account_owner,
        assert_account_role, assert_account_seeds, assert_external_account_data,
        make_owner_token_account_assertions, AccountRole,
    },
    cpi::{
        system::get_current_unix_timestamp,
        token::{TokenAccount, TokenMint, ORCA_MINT_ID, XORCA_MINT_ID},
    },
    error::ErrorCode,
    state::{pending_withdraw::PendingWithdraw, state::State},
    util::{
        account::{create_program_account, get_account_info},
        math::convert_xorca_to_orca,
    },
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::{instructions::Burn, ID as SPL_TOKEN_PROGRAM_ID};

pub fn process_instruction(
    accounts: &[AccountInfo],
    unstake_amount: &u64,
    withdraw_index: &u8,
) -> ProgramResult {
    let unstaker_account = get_account_info(accounts, 0)?;
    let state_account = get_account_info(accounts, 1)?;
    let vault_account = get_account_info(accounts, 2)?;
    let pending_withdraw_account = get_account_info(accounts, 3)?;
    let unstaker_xorca_ata = get_account_info(accounts, 4)?;
    let xorca_mint_account = get_account_info(accounts, 5)?;
    let orca_mint_account = get_account_info(accounts, 6)?;
    let system_program_account = get_account_info(accounts, 7)?;
    let token_program_account = get_account_info(accounts, 8)?;

    // 1. Unstaker Account Assertions
    assert_account_role(
        unstaker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. xOrca State Account Assertions
    assert_account_role(state_account, &[AccountRole::Writable])?;
    assert_account_owner(state_account, &crate::ID)?;
    let mut state_seeds = State::seeds();
    let state_bump = assert_account_seeds(state_account, &crate::ID, &state_seeds)?;
    state_seeds.push(Seed::from(&state_bump));
    let initial_escrowed_orca_amount = {
        // Use a block to control scope
        let state_ref = assert_account_data::<State>(state_account)?;
        state_ref.escrowed_orca_amount
    };

    // 3. Vault Account Assertions
    let vault_account_seeds = vec![
        Seed::from(state_account.key()),
        Seed::from(SPL_TOKEN_PROGRAM_ID.as_ref()),
        Seed::from(orca_mint_account.key()),
    ];
    assert_account_seeds(
        vault_account,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
        &vault_account_seeds,
    )?;
    let vault_account_data = assert_external_account_data::<TokenAccount>(vault_account)?;

    // 4. Pending Withdraw Account Assertions
    assert_account_role(pending_withdraw_account, &[AccountRole::Writable])?;
    assert_account_owner(pending_withdraw_account, &SYSTEM_PROGRAM_ID)?;
    let withdraw_index_bytes = [*withdraw_index];
    let mut pending_withdraw_seeds =
        PendingWithdraw::seeds(unstaker_account.key(), &withdraw_index_bytes);
    let pending_withdraw_bump = assert_account_seeds(
        pending_withdraw_account,
        &crate::ID,
        &pending_withdraw_seeds,
    )?;
    pending_withdraw_seeds.push(Seed::from(&pending_withdraw_bump));

    // 5. Unstaker LST Account assertions
    let unstaker_xorca_ata_data = make_owner_token_account_assertions(
        unstaker_xorca_ata,
        unstaker_account,
        xorca_mint_account,
    )?;
    if unstaker_xorca_ata_data.amount < *unstake_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    // 6. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_role(xorca_mint_account, &[AccountRole::Writable])?;
    assert_account_address(xorca_mint_account, &XORCA_MINT_ID)?;
    let xorca_mint_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;

    // 7. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;
    assert_external_account_data::<TokenMint>(orca_mint_account)?;

    // 8. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 9. Token Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Calculate withdrawable ORCA amount
    let non_escrowed_orca_amount = vault_account_data.amount - initial_escrowed_orca_amount;
    let withdrawable_orca_amount = convert_xorca_to_orca(
        *unstake_amount,
        non_escrowed_orca_amount,
        xorca_mint_data.supply,
    )?;

    // Burn unstaker's LST tokens
    let burn_instruction = Burn {
        mint: xorca_mint_account,
        account: unstaker_xorca_ata,
        authority: unstaker_account,
        amount: *unstake_amount,
    };
    burn_instruction.invoke()?;

    // Add the unstake ORCA amount to escrowed ORCA amount
    let mut state = assert_account_data_mut::<State>(state_account)?;
    state.escrowed_orca_amount += withdrawable_orca_amount;

    // Create new pending withdraw account
    let mut pending_withdraw_data = create_program_account::<PendingWithdraw>(
        system_program_account,
        unstaker_account,
        pending_withdraw_account,
        &[pending_withdraw_seeds.as_slice().into()],
    )?;

    // Populate pending withdraw account data
    pending_withdraw_data.withdrawable_orca_amount = withdrawable_orca_amount;
    let current_unix_timestamp = get_current_unix_timestamp()?;
    let withdrawable_timestamp = current_unix_timestamp
        .checked_add(state.cool_down_period_s)
        .ok_or(ErrorCode::ArithmeticError)?;
    pending_withdraw_data.withdrawable_timestamp = withdrawable_timestamp;

    Ok(())
}
