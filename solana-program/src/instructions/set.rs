use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        AccountRole,
    },
    error::ErrorCode,
    instructions::StateUpdateInstruction,
    state::state::State,
    util::account::get_account_info,
};
use pinocchio::{account_info::AccountInfo, ProgramResult};

pub fn process_instruction(
    accounts: &[AccountInfo],
    instruction_data: &StateUpdateInstruction,
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
    assert_account_owner(state_account, &crate::ID)?;
    // Use stored bump for verification - more efficient than assert_account_seeds
    let mut state_view = assert_account_data_mut::<State>(state_account)?;
    State::verify_address_with_bump(state_account, &crate::ID, state_view.bump)
        .map_err(|_| ErrorCode::InvalidSeeds)?;
    assert_account_address(update_authority_account, &state_view.update_authority)?;

    // Apply updates based on the instruction_data enum
    match instruction_data {
        StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s,
        } => {
            if *new_cool_down_period_s < 0 {
                return Err(ErrorCode::InvalidCoolDownPeriod.into());
            }
            state_view.cool_down_period_s = *new_cool_down_period_s;
        }
        StateUpdateInstruction::UpdateUpdateAuthority { new_authority } => {
            state_view.update_authority = *new_authority;
        }
    };

    Ok(())
}
