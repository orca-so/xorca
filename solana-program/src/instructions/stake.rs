use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, make_owner_token_account_assertions, AccountRole,
    },
    error::ErrorCode,
    state::{pending_claim::PendingClaim, staking_pool::StakingPool},
    util::account::get_account_info,
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::ID as SPL_TOKEN_PROGRAM_ID;

pub fn process_instruction(
    accounts: &[AccountInfo],
    stake_amount: &u64,
    claim_index: &u8,
) -> ProgramResult {
    let staker_account = get_account_info(accounts, 0)?;
    let staking_pool_account = get_account_info(accounts, 1)?;
    let staking_pool_stake_token_account = get_account_info(accounts, 2)?;
    let pending_claim_account = get_account_info(accounts, 3)?;
    let staker_stake_token_account = get_account_info(accounts, 4)?;
    let stake_token_mint_account = get_account_info(accounts, 5)?;
    let system_program_account = get_account_info(accounts, 6)?;
    let token_program_account = get_account_info(accounts, 7)?;

    // 1. Staker Account Assertions
    assert_account_role(
        staker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Staking Pool Account Assertions
    assert_account_role(staking_pool_account, &[AccountRole::Writable])?;
    assert_account_owner(staking_pool_account, &crate::ID)?;
    let mut staking_pool_data = assert_account_data_mut::<StakingPool>(staking_pool_account)?;

    // 3. Staking Pool Stake Token Account Assertions
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

    // 4. Pending Claim Account Assertions
    assert_account_role(pending_claim_account, &[AccountRole::Writable])?;
    assert_account_owner(pending_claim_account, &SYSTEM_PROGRAM_ID)?;
    let claim_index_bytes = [*claim_index];
    let mut pending_claim_seeds = PendingClaim::seeds(
        staking_pool_account.key(),
        staker_account.key(),
        &claim_index_bytes,
    );
    let pending_claim_bump =
        assert_account_seeds(pending_claim_account, &crate::ID, &pending_claim_seeds)?;
    pending_claim_seeds.push(Seed::from(&pending_claim_bump));

    // 5. Staker Stake Token Account Assertions
    let staker_stake_token_account_data = make_owner_token_account_assertions(
        staker_stake_token_account,
        staker_account,
        stake_token_mint_account,
    )?;
    if staker_stake_token_account_data.amount < *stake_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    // 6. Stake Token Mint Account Assertions
    assert_account_owner(stake_token_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(
        stake_token_mint_account,
        &staking_pool_data.stake_token_mint,
    )?;

    // 7. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 8. Token Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    Ok(())
}
