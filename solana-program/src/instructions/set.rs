use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_role, AccountRole,
    },
    state::state::State,
    util::account::get_account_info,
};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult};

pub fn process_instruction(
    accounts: &[AccountInfo],
    new_cool_down_period: &Option<u64>,
    new_update_authority: &Option<Pubkey>,
) -> ProgramResult {
    let update_authority_account = get_account_info(accounts, 0)?;
    let state_account = get_account_info(accounts, 1)?;

    // 1. Update Authority Account Assertions
    assert_account_role(
        update_authority_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. xOrca State Account Assertions
    assert_account_role(state_account, &[AccountRole::Writable])?;
    let mut state = assert_account_data_mut::<State>(state_account)?;
    assert_account_address(update_authority_account, &state.update_authority)?;

    // Apply updates if provided
    if let Some(period) = new_cool_down_period {
        state.cool_down_period_s = *period;
    }

    if let Some(new_auth) = new_update_authority {
        state.update_authority = *new_auth;
    }

    Ok(())
}
