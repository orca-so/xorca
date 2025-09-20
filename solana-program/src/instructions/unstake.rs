use crate::{
    assertions::account::{
        assert_account_address, assert_account_data, assert_account_data_mut, assert_account_owner,
        assert_account_role, assert_account_seeds, assert_external_account_data,
        make_owner_token_2022_account_assertions, make_owner_token_account_assertions, AccountRole,
    },
    cpi::{
        system::get_current_unix_timestamp,
        token::{
            Token2022Mint, TokenMint, ORCA_MINT_ID, SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
            XORCA_MINT_ID,
        },
    },
    error::ErrorCode,
    event::Event,
    state::{pending_withdraw::PendingWithdraw, state::State},
    util::{
        account::{create_program_account_secure, get_account_info},
        math::convert_xorca_to_orca,
    },
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token_2022::instructions::Burn;

pub fn process_instruction(
    accounts: &[AccountInfo],
    xorca_unstake_amount: &u64,
    withdraw_index: &u8,
) -> ProgramResult {
    let unstaker_account = get_account_info(accounts, 0)?;
    let state_account = get_account_info(accounts, 1)?;
    let pending_withdraw_account = get_account_info(accounts, 2)?;
    let unstaker_xorca_ata = get_account_info(accounts, 3)?;
    let xorca_mint_account = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let vault_account = get_account_info(accounts, 6)?;
    let system_program_account = get_account_info(accounts, 7)?;
    let spl_token_program_account = get_account_info(accounts, 8)?;
    let token_2022_program_account = get_account_info(accounts, 9)?;

    // 1. Unstaker Account Assertions
    assert_account_role(
        unstaker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Account Address Assertions
    assert_account_address(xorca_mint_account, &XORCA_MINT_ID)?;
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;
    assert_account_address(spl_token_program_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(token_2022_program_account, &TOKEN_2022_PROGRAM_ID)?;
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 2. xOrca State Account Assertions
    assert_account_role(state_account, &[AccountRole::Writable])?;
    assert_account_owner(state_account, &crate::ID)?;
    // No CPI signed by state in this instruction; only verify owner
    // We'll read the state data later when we need it

    // 3. Vault Account Assertions
    let vault_account_data = make_owner_token_account_assertions(
        vault_account,
        state_account,
        orca_mint_account,
        false,
    )?;

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
    let unstaker_xorca_ata_data = make_owner_token_2022_account_assertions(
        unstaker_xorca_ata,
        unstaker_account,
        xorca_mint_account,
        true,
    )?;
    if unstaker_xorca_ata_data.amount < *xorca_unstake_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    // 6. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &TOKEN_2022_PROGRAM_ID)?;
    assert_account_role(xorca_mint_account, &[AccountRole::Writable])?;
    let xorca_mint_data = assert_external_account_data::<Token2022Mint>(xorca_mint_account)?;
    // Enforce xORCA mint authority must be the state and freeze authority must be None
    if xorca_mint_data.mint_authority_flag == 0
        || xorca_mint_data.mint_authority != *state_account.key()
    {
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if xorca_mint_data.freeze_authority_flag != 0 {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // 7. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_external_account_data::<TokenMint>(orca_mint_account)?;

    // Calculate withdrawable ORCA amount using checked math
    let initial_escrowed_orca_amount = {
        let state_view = assert_account_data::<State>(state_account)?;

        // Verify state address using stored bump
        State::verify_address_with_bump(state_account, &crate::ID, state_view.bump)
            .map_err(|_| ErrorCode::InvalidSeeds)?;

        // Verify vault address using stored vault_bump
        State::verify_vault_address_with_bump(
            state_account,
            vault_account,
            orca_mint_account,
            state_view.vault_bump,
        )
        .map_err(|_| ErrorCode::InvalidSeeds)?;

        state_view.escrowed_orca_amount
    };

    let non_escrowed_orca_amount = vault_account_data
        .amount
        .checked_sub(initial_escrowed_orca_amount)
        .ok_or(ErrorCode::InsufficientVaultBacking)?;
    let withdrawable_orca_amount = convert_xorca_to_orca(
        *xorca_unstake_amount,
        non_escrowed_orca_amount,
        xorca_mint_data.supply,
    )?;

    if withdrawable_orca_amount == 0 {
        return Err(ErrorCode::InsufficientUnstakeAmount.into());
    }

    // Burn unstaker's LST tokens using Token2022 program
    let burn_instruction = Burn {
        token_program: &TOKEN_2022_PROGRAM_ID,
        mint: xorca_mint_account,
        account: unstaker_xorca_ata,
        authority: unstaker_account,
        amount: *xorca_unstake_amount,
    };
    burn_instruction.invoke()?;

    // Add the unstake ORCA amount to escrowed ORCA amount
    let mut state = assert_account_data_mut::<State>(state_account)?;
    state.escrowed_orca_amount = state
        .escrowed_orca_amount
        .checked_add(withdrawable_orca_amount)
        .ok_or(ErrorCode::ArithmeticError)?;

    // Create new pending withdraw account (secure against DoS attacks)
    let mut pending_withdraw_data = create_program_account_secure::<PendingWithdraw>(
        unstaker_account,
        pending_withdraw_account,
        &[pending_withdraw_seeds.as_slice().into()],
    )?;

    // Populate pending withdraw account data
    pending_withdraw_data.bump = pending_withdraw_bump[0];
    pending_withdraw_data.withdraw_index = *withdraw_index;
    pending_withdraw_data.unstaker = *unstaker_account.key();
    pending_withdraw_data.withdrawable_orca_amount = withdrawable_orca_amount;
    let current_unix_timestamp = get_current_unix_timestamp()?;
    let withdrawable_timestamp = current_unix_timestamp
        .checked_add(state.cool_down_period_s)
        .ok_or(ErrorCode::CoolDownOverflow)?;
    pending_withdraw_data.withdrawable_timestamp = withdrawable_timestamp;

    let final_vault_amount = vault_account_data.amount;
    let final_xorca_supply = xorca_mint_data.supply - *xorca_unstake_amount;

    Event::Unstake {
        xorca_unstake_amount: xorca_unstake_amount,
        vault_orca_amount: &final_vault_amount,
        vault_escrowed_orca_amount: &state.escrowed_orca_amount,
        xorca_mint_supply: &final_xorca_supply,
        withdrawable_orca_amount: &withdrawable_orca_amount,
        cool_down_period_s: &state.cool_down_period_s,
        withdraw_index: withdraw_index,
    }
    .emit()?;

    Ok(())
}
