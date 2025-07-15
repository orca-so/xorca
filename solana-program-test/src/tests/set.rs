use crate::{
    assert_program_error, assert_program_success, state_data, TestContext, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_state_address, Set, SetInstructionArgs, State, StateUpdateInstruction,
    XorcaStakingProgramError,
};

/// Sets up the basic test context with correct PDAs
fn setup_base_set_context(
    ctx: &mut TestContext,
    initial_update_authority: Pubkey,
) -> (Pubkey, Pubkey) {
    let state_account_pda = find_state_address().unwrap().0;
    ctx.write_account(
        state_account_pda,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => initial_update_authority,
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    (state_account_pda, initial_update_authority)
}

// --- Invalid Account Configuration Helpers ---
fn make_state_account_invalid_owner(ctx: &mut TestContext, state_account: Pubkey) {
    ctx.write_account(
        state_account,
        Pubkey::new_unique(), // Incorrect owner
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
}

#[test]
fn test_set_cool_down_period_success() {
    let mut ctx = TestContext::new();
    let initial_update_authority = ctx.signer();
    let (state_account, update_authority_signer) =
        setup_base_set_context(&mut ctx, initial_update_authority);
    let new_cool_down_period: i64 = 30 * 24 * 60 * 60;
    let ix = Set {
        update_authority_account: update_authority_signer,
        state_account: state_account,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_period: new_cool_down_period,
        },
    });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    assert_eq!(
        state_account_after.data.cool_down_period_s, new_cool_down_period,
        "Cool down period should be updated"
    );
    assert_eq!(
        state_account_after.data.update_authority, initial_update_authority,
        "Update authority should remain unchanged"
    );
}

#[test]
fn test_set_update_authority_success() {
    let mut ctx = TestContext::new();
    let initial_update_authority = ctx.signer();
    let (state_account, update_authority_signer) =
        setup_base_set_context(&mut ctx, initial_update_authority);
    let new_update_authority = Pubkey::new_unique();
    let ix = Set {
        update_authority_account: update_authority_signer,
        state_account: state_account,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateUpdateAuthority {
            new_authority: new_update_authority,
        },
    });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    assert_eq!(
        state_account_after.data.update_authority, new_update_authority,
        "Update authority should be updated"
    );
    assert_eq!(
        state_account_after.data.cool_down_period_s,
        7 * 24 * 60 * 60,
        "Cool down period should remain unchanged"
    );
}

#[test]
fn test_set_invalid_update_authority_mismatch() {
    let mut ctx = TestContext::new();
    let initial_update_authority = Pubkey::new_unique();
    let (state_account, _original_signer) =
        setup_base_set_context(&mut ctx, initial_update_authority);
    let wrong_signer = ctx.signer();
    let ix = Set {
        update_authority_account: wrong_signer,
        state_account: state_account,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod { new_period: 100 },
    });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

#[test]
fn test_set_invalid_state_account_owner() {
    let mut ctx = TestContext::new();
    let initial_update_authority = ctx.signer();
    let (state_account, update_authority_signer) =
        setup_base_set_context(&mut ctx, initial_update_authority);
    make_state_account_invalid_owner(&mut ctx, state_account);
    let ix = Set {
        update_authority_account: update_authority_signer,
        state_account: state_account,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod { new_period: 100 },
    });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_set_invalid_state_account_seeds() {
    let mut ctx = TestContext::new();
    let initial_update_authority = ctx.signer();
    let (correct_state_account, update_authority_signer) =
        setup_base_set_context(&mut ctx, initial_update_authority);
    ctx.write_account(
        correct_state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => initial_update_authority,
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    // let invalid_state_account = make_state_account_invalid_seeds(&mut ctx);
    let ix = Set {
        update_authority_account: update_authority_signer,
        state_account: Pubkey::new_unique(),
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod { new_period: 100 },
    });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}
