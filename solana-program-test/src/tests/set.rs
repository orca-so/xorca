use crate::{assert_program_error, TestContext};
use xorca::{
    find_state_address, Set, SetInstructionArgs, State, StateUpdateInstruction,
    XorcaStakingProgramError,
};

#[test]
fn set_updates_cooldown() {
    let mut ctx = TestContext::new();
    let (state, state_bump) = find_state_address().unwrap();
    // Seed state with update authority as signer
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => state_bump,
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
    let (state, state_bump) = find_state_address().unwrap();
    // Seed state with current update authority as signer
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => state_bump,
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
    let (state, state_bump) = find_state_address().unwrap();
    // Seed state with different authority than signer
    let wrong_account = solana_sdk::pubkey::Pubkey::new_unique();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => wrong_account,
            cool_down_period_s => 10,
            bump => state_bump,
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
    let (state, state_bump) = find_state_address().unwrap();
    ctx.write_account(
        state,
        solana_sdk::system_program::ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => state_bump,
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
            bump => 0, // Wrong bump for bogus state
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
    let (state, state_bump) = find_state_address().unwrap();
    // Seed proper state
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => state_bump,
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
    let (state, state_bump) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 777,
            bump => state_bump,
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
    let (state, state_bump) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => state_bump,
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

// Test setting cooldown to large positive value (i64::MAX)
#[test]
fn set_updates_cooldown_to_max_success() {
    let mut ctx = TestContext::new();
    let (state, state_bump) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 0,
            bump => state_bump,
        ),
    )
    .unwrap();
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
}

// Test setting cooldown to zero succeeds
#[test]
fn set_updates_cooldown_to_zero_success() {
    let mut ctx = TestContext::new();
    let (state, state_bump) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 1, // Start with non-zero to test setting to zero
            bump => state_bump,
        ),
    )
    .unwrap();
    let ix_zero = Set {
        update_authority_account: ctx.signer(),
        state_account: state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 0,
        },
    });
    assert!(ctx.send(ix_zero).is_ok());
    let st_zero = ctx.get_account::<State>(state).unwrap();
    assert_eq!(st_zero.data.cool_down_period_s, 0);
}

// Test setting negative cooldown fails
#[test]
fn set_fails_on_negative_cooldown_fails() {
    let mut ctx = TestContext::new();
    let (state, state_bump) = find_state_address().unwrap();
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 0,
            bump => state_bump,
        ),
    )
    .unwrap();
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
    assert_program_error!(res, XorcaStakingProgramError::InvalidCoolDownPeriod);
}

// Test that the new verification method with stored bumps rejects wrong bumps
#[test]
fn set_fails_with_wrong_bump_in_state_data() {
    let mut ctx = TestContext::new();
    
    // Create a bogus state account with wrong address but correct owner
    let bogus_state = solana_sdk::pubkey::Pubkey::new_unique();
    
    // Use wrong bump in account data
    let wrong_bump = 0; // Wrong bump for bogus state
    ctx.write_account(
        bogus_state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => wrong_bump, // Wrong bump in account data
        ),
    )
    .unwrap();
    
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: bogus_state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 500,
        },
    });
    
    let res = ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Test that the new verification method works correctly with correct bump
#[test]
fn set_succeeds_with_correct_bump_in_state_data() {
    let mut ctx = TestContext::new();
    let (state, correct_bump) = find_state_address().unwrap();
    
    // Use correct bump in account data
    ctx.write_account(
        state,
        xorca::XORCA_STAKING_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => ctx.signer(),
            cool_down_period_s => 10,
            bump => correct_bump, // Correct bump in account data
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
    
    let res = ctx.send(ix);
    assert!(res.is_ok());
    
    // Verify the update was applied
    let state_account = ctx.get_account::<State>(state).unwrap();
    assert_eq!(state_account.data.cool_down_period_s, 500);
}
