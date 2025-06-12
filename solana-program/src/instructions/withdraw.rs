use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, make_owner_token_account_assertions, AccountRole,
    },
    cpi::system::get_current_unix_timestamp,
    error::ErrorCode,
    state::{pending_withdraw::PendingWithdraw, staking_pool::StakingPool},
    util::account::get_account_info,
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::{instructions::Transfer, ID as SPL_TOKEN_PROGRAM_ID};

pub fn process_instruction(accounts: &[AccountInfo], withdraw_index: &u8) -> ProgramResult {
    let unstaker_account = get_account_info(accounts, 0)?;
    let staking_pool_account = get_account_info(accounts, 1)?;
    let pending_withdraw_account = get_account_info(accounts, 2)?;
    let unstaker_stake_token_account = get_account_info(accounts, 3)?;
    let staking_pool_stake_token_account = get_account_info(accounts, 4)?;
    let stake_token_mint_account = get_account_info(accounts, 5)?;
    let system_program_account = get_account_info(accounts, 6)?;
    let token_program_account = get_account_info(accounts, 7)?;

    // 1. Unstaker Account Assertions
    assert_account_role(
        unstaker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Staking Pool Account Assertions
    assert_account_role(staking_pool_account, &[AccountRole::Writable])?;
    assert_account_owner(staking_pool_account, &crate::ID)?;
    let mut staking_pool_seeds = StakingPool::seeds(stake_token_mint_account.key());
    let staking_pool_bump =
        assert_account_seeds(staking_pool_account, &crate::ID, &staking_pool_seeds)?;
    staking_pool_seeds.push(Seed::from(&staking_pool_bump));
    let mut staking_pool_data = assert_account_data_mut::<StakingPool>(staking_pool_account)?;

    // 3. Pending Withdraw Account Assertions
    assert_account_role(pending_withdraw_account, &[AccountRole::Writable])?;
    assert_account_owner(pending_withdraw_account, &crate::ID)?;
    let withdraw_index_bytes = [*withdraw_index];
    let pending_withdraw_seeds = PendingWithdraw::seeds(
        staking_pool_account.key(),
        unstaker_account.key(),
        &withdraw_index_bytes,
    );
    assert_account_seeds(
        pending_withdraw_account,
        &crate::ID,
        &pending_withdraw_seeds,
    )?;
    let pending_withdraw_data =
        assert_account_data_mut::<PendingWithdraw>(pending_withdraw_account)?;

    // 4. Unstaker Stake Token Account Assertions
    make_owner_token_account_assertions(
        unstaker_stake_token_account,
        unstaker_account,
        stake_token_mint_account,
    )?;

    // 5. Staking Pool Stake Token Account Assertions
    let staking_pool_stake_token_seeds = vec![
        Seed::from(staking_pool_account.key()),
        Seed::from(SPL_TOKEN_PROGRAM_ID.as_ref()),
        Seed::from(stake_token_mint_account.key()),
    ];
    assert_account_seeds(
        staking_pool_stake_token_account,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
        &staking_pool_stake_token_seeds,
    )?;

    // 6. Stake Token Mint Account Assertions
    assert_account_owner(stake_token_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(
        stake_token_mint_account,
        &staking_pool_data.stake_token_mint,
    )?;

    // 7. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 8. Token Program Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Validate pending withdraw
    let current_unix_timestamp = get_current_unix_timestamp()?;
    if current_unix_timestamp < pending_withdraw_data.withdrawable_timestamp {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // Transfer withdrawable stake tokens from staking pool to unstaker
    let transfer_instruction = Transfer {
        from: staking_pool_stake_token_account,
        to: unstaker_stake_token_account,
        authority: staking_pool_account,
        amount: pending_withdraw_data.withdrawable_stake_amount,
    };
    transfer_instruction.invoke_signed(&[staking_pool_seeds.as_slice().into()])?;

    // Remove tokens from escrow
    staking_pool_data.escrowed_stake_token_amount -=
        pending_withdraw_data.withdrawable_stake_amount;

    // TODO: Close Pending Withdraw Account
    Ok(())
}
