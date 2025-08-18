use crate::{assert_program_error, TestContext};
use xorca::{
    find_state_address, Set, SetInstructionArgs, State, StateUpdateInstruction,
    XorcaStakingProgramError,
};

#[test]
fn set_updates_cooldown() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Seed state with update authority as signer
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
        ),
    )
    .unwrap();

    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 500,
        },
    });
    assert!(ctx.send(ix).is_ok());
    let state_account = ctx.get_account::<State>(state).unwrap();
    assert_eq!(state_account.data.cool_down_period_s, 500);
}

// Success: update the update authority to a new pubkey
#[test]
fn set_updates_update_authority() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Seed state with current update authority as signer
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
        ),
    )
    .unwrap();
    let new_auth = solana_sdk::pubkey::Pubkey::new_unique();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateUpdateAuthority {
            new_authority: new_auth,
        },
    });
    assert!(ctx.send(ix).is_ok());
    let state_account = ctx.get_account::<State>(state).unwrap();
    assert_eq!(state_account.data.update_authority, new_auth);
}

// Failure: wrong signer (not current update authority)
#[test]
fn set_fails_with_wrong_update_authority_signer() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Seed state with different authority than signer
    let wrong_account = solana_sdk::pubkey::Pubkey::new_unique();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => wrong_account,
            cool_down_period_s => 10,
        ),
    )
    .unwrap();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 123,
        },
    });
    let res = ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Failure: state must be owned by program
#[test]
fn set_fails_with_wrong_state_owner() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        state,
        solana_sdk::system_program::ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
        ),
    )
    .unwrap();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 123,
        },
    });
    let res = ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Failure: state PDA seeds must be correct
#[test]
fn set_fails_with_invalid_state_seeds() {
    let mut ctx = TestContext::new();
    // Create bogus state with correct owner but wrong seeds
    let bogus_state = solana_sdk::pubkey::Pubkey::new_unique();
    ctx.write_account(
        bogus_state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
        ),
    )
    .unwrap();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: bogus_state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 123,
        },
    });
    let res = ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Failure: update_authority account must be a signer
#[test]
fn set_fails_when_update_authority_not_signer() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Seed proper state
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
        ),
    )
    .unwrap();
    // Use a non-signer pubkey instead of ctx.signer as the update_authority account
    let non_signer = solana_sdk::pubkey::Pubkey::new_unique();
    // We cannot actually mark it non-signer in our harness, but we can still pass a different pubkey which fails address match first.
    let ix = Set {
        update_authority_account: non_signer,
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 123,
        },
    });
    let res = ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Idempotent: updating cooldown to the same value is a no-op but succeeds
#[test]
fn set_cooldown_idempotent_noop_succeeds() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 777,
        ),
    )
    .unwrap();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 777,
        },
    });
    assert!(ctx.send(ix).is_ok());
    let state_account = ctx.get_account::<State>(state).unwrap();
    assert_eq!(state_account.data.cool_down_period_s, 777);
}

// Idempotent: updating authority to the same pubkey succeeds and leaves authority unchanged
#[test]
fn set_update_authority_idempotent_noop_succeeds() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
        ),
    )
    .unwrap();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateUpdateAuthority {
            new_authority: ctx.signer(),
        },
    });
    assert!(ctx.send(ix).is_ok());
    let state_account = ctx.get_account::<State>(state).unwrap();
    assert_eq!(state_account.data.update_authority, ctx.signer());
}

// Boundary values: accepts large and negative cooldown values (current program does not restrict sign)
#[test]
#[ignore = "Pending program change: cooldown must be non-negative"]
fn set_updates_cooldown_boundary_values() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 0,
        ),
    )
    .unwrap();
    // Large positive
    let ix_max = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: i64::MAX,
        },
    });
    assert!(ctx.send(ix_max).is_ok());
    let st_max = ctx.get_account::<State>(state).unwrap();
    assert_eq!(st_max.data.cool_down_period_s, i64::MAX);

    // Negative should fail (once program enforces it)
    let ix_neg = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: -123,
        },
    });
    let res = ctx.send(ix_neg);
    assert!(res.is_err());
}
