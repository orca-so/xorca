use crate::{
    assertions::account::{
        assert_account_address, assert_account_data, assert_account_data_mut, assert_account_owner,
        assert_account_role, assert_account_seeds, make_owner_token_account_assertions,
        AccountRole,
    },
    cpi::{system::get_current_unix_timestamp, token::ORCA_MINT_ID},
    error::ErrorCode,
    event::Event,
    state::{pending_withdraw::PendingWithdraw, state::State},
    util::account::{close_program_account, get_account_info},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::{instructions::Transfer, ID as SPL_TOKEN_PROGRAM_ID};

pub fn process_instruction(accounts: &[AccountInfo], withdraw_index: &u8) -> ProgramResult {
    let unstaker_account = get_account_info(accounts, 0)?;
    let state_account = get_account_info(accounts, 1)?;
    let pending_withdraw_account = get_account_info(accounts, 2)?;
    let unstaker_orca_ata = get_account_info(accounts, 3)?;
    let vault_account = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let system_program_account = get_account_info(accounts, 6)?;
    let token_program_account = get_account_info(accounts, 7)?;

    // 1. Unstaker Account Assertions
    assert_account_role(
        unstaker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Xorca State Account Assertions
    assert_account_role(state_account, &[AccountRole::Writable])?;
    assert_account_owner(state_account, &crate::ID)?;
    let mut state_seeds = State::seeds();
    let state_bump_value: u8 = {
        let view = assert_account_data::<State>(state_account)?;
        view.bump
    };

    State::verify_address_with_bump(state_account, &crate::ID, state_bump_value)
        .map_err(|_| ErrorCode::InvalidSeeds)?;

    let bump_bytes = [state_bump_value];
    state_seeds.push(Seed::from(&bump_bytes));

    // 3. Pending Withdraw Account Assertions
    assert_account_role(pending_withdraw_account, &[AccountRole::Writable])?;
    assert_account_owner(pending_withdraw_account, &crate::ID)?;
    let withdraw_index_bytes = [*withdraw_index];
    // Prefer cached bump from account data if present; else compute and verify
    let pending_withdraw_bump_byte = {
        let data = assert_account_data::<PendingWithdraw>(pending_withdraw_account)?;
        data.bump
    };

    // Use derive_address for verification when we have the stored bump
    PendingWithdraw::verify_address_with_bump(
        pending_withdraw_account,
        unstaker_account.key(),
        &withdraw_index_bytes,
        &crate::ID,
        pending_withdraw_bump_byte,
    )
    .map_err(|_| ErrorCode::InvalidSeeds)?;

    let (withdrawable_orca_amount, withdrawable_timestamp) = {
        let pending_withdraw_data =
            assert_account_data::<PendingWithdraw>(pending_withdraw_account)?;
        (
            pending_withdraw_data.withdrawable_orca_amount,
            pending_withdraw_data.withdrawable_timestamp,
        )
    };

    // 4. Unstaker Stake Token Account Assertions
    make_owner_token_account_assertions(unstaker_orca_ata, unstaker_account, orca_mint_account)?;

    // 5. Vault Account Assertions
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
    make_owner_token_account_assertions(vault_account, state_account, orca_mint_account)?;

    // 6. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;

    // 7. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 8. Token Program Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Validate pending withdraw
    let current_unix_timestamp = get_current_unix_timestamp()?;
    if current_unix_timestamp < withdrawable_timestamp {
        return Err(ErrorCode::CoolDownPeriodStillActive.into());
    }

    // Pre-check escrow underflow before CPI
    {
        let state_data = assert_account_data::<State>(state_account)?;
        if state_data.escrowed_orca_amount < withdrawable_orca_amount {
            return Err(ErrorCode::InsufficientEscrow.into());
        }
    }

    // Transfer withdrawable stake tokens from xOrca state ATA to unstaker ATA
    let transfer_instruction = Transfer {
        from: vault_account,
        to: unstaker_orca_ata,
        authority: state_account,
        amount: withdrawable_orca_amount,
    };
    transfer_instruction.invoke_signed(&[state_seeds.as_slice().into()])?;

    // Close the pending_withdraw account and refund lamports to unstaker
    close_program_account(pending_withdraw_account, unstaker_account)?;

    // Remove tokens from escrow
    let mut state = assert_account_data_mut::<State>(state_account)?;
    state.escrowed_orca_amount = state
        .escrowed_orca_amount
        .checked_sub(withdrawable_orca_amount)
        .ok_or(ErrorCode::InsufficientEscrow)?;

    Event::Withdraw {
        vault_escrowed_orca_amount: &state.escrowed_orca_amount,
        withdrawable_orca_amount: &withdrawable_orca_amount,
        cool_down_period_s: &state.cool_down_period_s,
        withdraw_index: withdraw_index,
    }
    .emit()?;

    Ok(())
}
