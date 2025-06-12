use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, make_owner_token_account_assertions, AccountRole,
    },
    state::{pending_withdraw::PendingWithdraw, xorca_state::XorcaState},
    util::account::get_account_info,
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::ID as SPL_TOKEN_PROGRAM_ID;

pub fn process_instruction(accounts: &[AccountInfo], withdraw_index: &u8) -> ProgramResult {
    let unstaker_account = get_account_info(accounts, 0)?;
    let xorca_state_account = get_account_info(accounts, 1)?;
    let pending_withdraw_account = get_account_info(accounts, 2)?;
    let unstaker_orca_ata = get_account_info(accounts, 3)?;
    let xorca_state_orca_ata = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let system_program_account = get_account_info(accounts, 6)?;
    let token_program_account = get_account_info(accounts, 7)?;

    // 1. Unstaker Account Assertions
    assert_account_role(
        unstaker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Staking Pool Account Assertions
    assert_account_role(xorca_state_account, &[AccountRole::Writable])?;
    assert_account_owner(xorca_state_account, &crate::ID)?;
    let mut xorca_state_seeds = XorcaState::seeds(orca_mint_account.key());
    let xorca_state_bump =
        assert_account_seeds(xorca_state_account, &crate::ID, &xorca_state_seeds)?;
    xorca_state_seeds.push(Seed::from(&xorca_state_bump));
    let mut xorca_state = assert_account_data_mut::<XorcaState>(xorca_state_account)?;

    // 3. Pending Withdraw Account Assertions
    assert_account_role(pending_withdraw_account, &[AccountRole::Writable])?;
    assert_account_owner(pending_withdraw_account, &crate::ID)?;
    let withdraw_index_bytes = [*withdraw_index];
    let pending_withdraw_seeds = PendingWithdraw::seeds(
        xorca_state_account.key(),
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
    make_owner_token_account_assertions(unstaker_orca_ata, unstaker_account, orca_mint_account)?;

    // 5. Staking Pool Stake Token Account Assertions
    let staking_pool_stake_token_seeds = vec![
        Seed::from(xorca_state_account.key()),
        Seed::from(SPL_TOKEN_PROGRAM_ID.as_ref()),
        Seed::from(orca_mint_account.key()),
    ];
    assert_account_seeds(
        xorca_state_orca_ata,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
        &staking_pool_stake_token_seeds,
    )?;

    // 6. Stake Token Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(orca_mint_account, &xorca_state.orca_mint)?;

    // 7. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 8. Token Program Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    Ok(())
}
