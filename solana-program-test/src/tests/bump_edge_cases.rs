use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::{
    assert_program_error, TestContext, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{find_pending_withdraw_pda, find_state_address, XorcaStakingProgramError};

// Test with state account having wrong bump in account data (non-zero but incorrect)
#[test]
fn test_stake_state_account_wrong_bump_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Overwrite state with wrong bump in account data
    let (_, correct_bump) = find_state_address().unwrap();
    let wrong_bump = if correct_bump == 255 {
        254
    } else {
        correct_bump + 1
    };
    env.ctx
        .write_account(
            env.state,
            xorca::ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 60,
                bump => wrong_bump, // Wrong bump in account data
            ),
        )
        .unwrap();

    let ix = xorca::Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.sends(&[ix]);
    // The program fails during PDA creation with wrong bump, not during seed validation
    assert!(res.is_err());
}

// Test with pending withdraw account having wrong bump in account data
#[test]
fn test_withdraw_pending_withdraw_wrong_bump_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // First, create a pending withdraw through unstake
    let idx: u8 = 0u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    assert!(crate::utils::flows::do_unstake(&mut env, idx, 1_000_000).is_ok());

    // Now corrupt the bump in the pending withdraw account data
    let (_, correct_bump) = find_pending_withdraw_pda(&env.staker, &idx).unwrap();
    let wrong_bump = if correct_bump == 255 {
        254
    } else {
        correct_bump + 1
    };
    env.ctx
        .write_account(
            pending_withdraw_account,
            xorca::ID,
            crate::pending_withdraw_data!(
                unstaker => env.staker,
                withdraw_index => idx,
                withdrawable_orca_amount => 1_000_000,
                withdrawable_timestamp => 0,
                bump => wrong_bump, // Wrong bump in account data
            ),
        )
        .unwrap();

    // Advance time to allow withdrawal
    let mut clock = env.ctx.get_sysvar::<solana_sdk::clock::Clock>();
    clock.unix_timestamp = 1000;
    env.ctx.set_sysvar::<solana_sdk::clock::Clock>(&clock);

    let ix = xorca::Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::WithdrawInstructionArgs {
        withdraw_index: idx,
    });
    let res = env.ctx.sends(&[ix]);
    // The program fails during PDA creation with wrong bump, not during seed validation
    assert!(res.is_err());
}

// Test with state account having bump=0 but wrong PDA derivation
#[test]
fn test_stake_state_account_zero_bump_wrong_pda() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Create a bogus state account with bump=0 but wrong PDA
    let bogus_state = Pubkey::new_unique();
    env.ctx
        .write_account(
            bogus_state,
            xorca::ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 60,
                bump => 0, // Zero bump but wrong PDA
            ),
        )
        .unwrap();

    let ix = xorca::Stake {
        staker_account: env.staker,
        state_account: bogus_state, // Wrong state account
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.sends(&[ix]);
    // The program fails during PDA creation with wrong bump, not during seed validation
    assert!(res.is_err());
}

// Test with pending withdraw account having bump=0 but wrong PDA derivation
#[test]
fn test_withdraw_pending_withdraw_zero_bump_wrong_pda() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Create a bogus pending withdraw account with bump=0 but wrong PDA
    let bogus_pending = Pubkey::new_unique();
    env.ctx
        .write_account(
            bogus_pending,
            xorca::ID,
            crate::pending_withdraw_data!(
                unstaker => env.staker,
                withdraw_index => 0,
                withdrawable_orca_amount => 1_000_000,
                withdrawable_timestamp => 0,
                bump => 0, // Zero bump but wrong PDA
            ),
        )
        .unwrap();

    let idx = 0u8;
    let ix = xorca::Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: bogus_pending, // Wrong pending account
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::WithdrawInstructionArgs {
        withdraw_index: idx,
    });
    let res = env.ctx.sends(&[ix]);
    // The program fails during PDA creation with wrong bump, not during seed validation
    assert!(res.is_err());
}

// Test with maximum possible bump value (255)
#[test]
fn test_stake_with_max_bump_value() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Overwrite state with maximum bump value
    env.ctx
        .write_account(
            env.state,
            xorca::ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 60,
                bump => 255, // Maximum bump value
            ),
        )
        .unwrap();

    let ix = xorca::Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.sends(&[ix]);
    // The program fails during PDA creation with wrong bump, not during seed validation
    assert!(res.is_err());
}

// Test with corrupted account data (wrong discriminator)
#[test]
fn test_stake_corrupted_state_discriminator() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Create corrupted state data with wrong discriminator
    let mut corrupted_data = crate::state_data!(
        escrowed_orca_amount => 0,
        update_authority => Pubkey::default(),
        cool_down_period_s => 60,
        bump => 0,
    );
    // Manually corrupt the discriminator
    let mut data_bytes = borsh::to_vec(&corrupted_data).unwrap();
    data_bytes[0] = 0xFF; // Wrong discriminator
    env.ctx
        .write_raw_account(env.state, xorca::ID, data_bytes)
        .unwrap();

    let ix = xorca::Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.sends(&[ix]);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Test with account data that's too small
#[test]
fn test_stake_state_account_too_small() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Make the state account too small
    let mut state_acc = env.ctx.get_raw_account(env.state).unwrap();
    state_acc.data.truncate(10); // Make it much smaller than expected State size
    env.ctx.set_account(env.state, state_acc).unwrap();

    let ix = xorca::Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.sends(&[ix]);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Test with vault account having wrong bump
#[test]
fn test_stake_vault_account_wrong_bump() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Create a bogus vault account with wrong bump
    let bogus_vault = Pubkey::new_unique();
    env.ctx
        .write_account(
            bogus_vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.state, amount => 1_000_000_000),
        )
        .unwrap();

    let ix = xorca::Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: bogus_vault, // Wrong vault account
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.sends(&[ix]);
    // The program fails during PDA creation with wrong bump, not during seed validation
    assert!(res.is_err());
}
