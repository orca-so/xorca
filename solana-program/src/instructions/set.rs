use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, AccountRole,
    },
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
    let state_seeds = State::seeds();
    assert_account_seeds(state_account, &crate::ID, &state_seeds)?;
    let mut state = assert_account_data_mut::<State>(state_account)?;
    assert_account_address(update_authority_account, &state.update_authority)?;

    // Apply updates based on the instruction_data enum
    match instruction_data {
        StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s,
        } => {
            state.cool_down_period_s = *new_cool_down_period_s;
        }
        StateUpdateInstruction::UpdateUpdateAuthority { new_authority } => {
            state.update_authority = *new_authority;
        }
    };

    Ok(())
}
