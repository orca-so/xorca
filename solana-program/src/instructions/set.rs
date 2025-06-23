use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_role, AccountRole,
    },
    state::staking_pool::StakingPool,
    util::account::get_account_info,
};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult};

pub fn process_instruction(
    accounts: &[AccountInfo],
    new_wind_up_period: &Option<u64>,
    new_cool_down_period: &Option<u64>,
    new_update_authority: &Option<Pubkey>,
) -> ProgramResult {
    let update_authority_account = get_account_info(accounts, 0)?;
    let staking_pool_account = get_account_info(accounts, 1)?;

    // 1. Update Authority Account Assertions
    assert_account_role(
        update_authority_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Staking Pool Account Assertions
    assert_account_role(staking_pool_account, &[AccountRole::Writable])?;
    let mut staking_pool_data = assert_account_data_mut::<StakingPool>(staking_pool_account)?;
    assert_account_address(
        update_authority_account,
        &staking_pool_data.update_authority,
    )?;

    // Apply updates if provided
    if let Some(period) = new_wind_up_period {
        staking_pool_data.wind_up_period_s = *period;
    }

    if let Some(period) = new_cool_down_period {
        staking_pool_data.cool_down_period_s = *period;
    }

    if let Some(new_auth) = new_update_authority {
        staking_pool_data.update_authority = *new_auth;
    }

    Ok(())
}
