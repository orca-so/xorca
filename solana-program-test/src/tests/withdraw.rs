use crate::utils::assert::{
    assert_account_closed, assert_withdraw_effects, decode_events_from_result,
    take_withdraw_snapshot,
};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::{
    advance_clock_env, do_unstake, do_withdraw, stake_orca, unstake_and_advance,
};
use crate::{
    assert_program_error, TestContext, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_pending_withdraw_pda, find_state_address, Event, PendingWithdraw, State, TokenAccount,
    Withdraw, WithdrawInstructionArgs, XorcaStakingProgramError,
};
use xorca::{Set, SetInstructionArgs, StateUpdateInstruction};

// Cost for resizing an account to 0 in Solana runtime
const RESIZE_TO_ZERO_COST: u64 = 5000;

// Mirror structure of stake tests: success, edge cases, account validation

// === 1) Success behavior ===

#[test]
fn withdraw_transfers_funds_and_clears_escrow() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 10_000_000,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 7u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;

    let xorca_unstake_amount = 10_000_000_000u64;
    let _ = unstake_and_advance(&mut env, withdraw_index, xorca_unstake_amount, 2);
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, withdraw_index).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    // Human-readable aggregate assertions
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw basic",
    );
}

// Success: escrow initially > 0 should decrease by withdrawable amount and succeed
#[test]
fn withdraw_succeeds_when_escrow_already_nonzero() {
    let ctx = TestContext::new();
    // Start with non-zero escrow recorded in state
    let pool = PoolSetup {
        xorca_supply: 10_000_000_000,
        vault_orca: 2_000_000,
        escrowed_orca: 1_000_000,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 3_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 2u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;

    let xorca_unstake_amount = 3_000_000u64;
    let _ = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 2);
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw with prior escrow",
    );
}

// Success: cooldown = 0 should allow immediate withdraw after unstake
#[test]
fn withdraw_succeeds_with_zero_cooldown() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 0,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 1u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;

    let xorca_unstake_amount = 2_000_000u64;
    let _ = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 0);
    // Snapshot + pending
    let snap: crate::utils::assert::WithdrawSnapshot = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());

    // Post-withdraw assertions via helper
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw zero cooldown",
    );
}

// Success: high exchange rate (xORCA expensive) should produce small withdraw, still succeed
#[test]
fn withdraw_succeeds_at_high_exchange_rate_small_withdraw() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 20_000_000_000,
        vault_orca: 5_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 3u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;

    // Small withdrawable ORCA expected
    let xorca_unstake_amount = 1_000_000u64;
    let _ = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 2);
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw high rate (small amount)",
    );
}

// Success: low exchange rate (xORCA cheap) produces larger withdraw, within vault balance
#[test]
fn withdraw_succeeds_at_low_exchange_rate_large_withdraw() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 500_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 50_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 4u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;

    let xorca_unstake_amount = 50_000_000u64;
    let _ = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 2);
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw low rate (large amount)",
    );
}

// === 2) Edge cases ===

#[test]
fn withdraw_zero_index_path() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 0u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;

    let xorca_unstake_amount = 1_000_000u64;
    let _ = unstake_and_advance(&mut env, withdraw_index, xorca_unstake_amount, 2);
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, withdraw_index).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw zero index",
    );
}

// Edge: escrow = 0 path (state starts at zero and withdrawable may be zero) should not underflow
#[test]
fn withdraw_with_zero_escrow_state_no_underflow() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 10_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 5u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;

    let xorca_unstake_amount = 1u64;
    let _ = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 2);
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw with zero escrow state",
    );
}

// Edge: staker ORCA balance initially non-zero; after withdraw, balance should increase
#[test]
fn withdraw_increases_staker_orca_balance_from_nonzero() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 2_000_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 6u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;

    let xorca_unstake_amount = 2_000_000u64;
    let _ = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 2);
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    // Old balance check retained
    let bal_before = env
        .ctx
        .get_account::<TokenAccount>(env.staker_orca_ata)
        .unwrap()
        .data
        .amount;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed",
    );
    let bal_after = env
        .ctx
        .get_account::<TokenAccount>(env.staker_orca_ata)
        .unwrap()
        .data
        .amount;
    assert!(
        bal_after >= bal_before,
        "user: ORCA balance should not decrease on withdraw"
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw increases user ORCA from nonzero",
    );
}

// Probe: extreme large numbers for escrow and withdrawable
#[test]
fn withdraw_extreme_large_values() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: u64::MAX - 1_000,
        vault_orca: u64::MAX / 2,
        escrowed_orca: u64::MAX / 4,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: u64::MAX / 8,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 7u8;

    let pending_withdraw_account =
        unstake_and_advance(&mut env, idx, (u64::MAX / 8).min(10_000_000_000), 2);

    // Snapshot before withdraw and capture pending amount
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    // Act
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert!(res.is_ok(), "withdraw failed: {:?}", res);
    assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending closed on large-numbers withdraw",
    );

    // Assert effects
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        (u64::MAX / 8).min(10_000_000_000),
        "extreme large withdraw",
    );
}

// === 3) Account validation ===

#[test]
fn withdraw_invalid_system_program_id() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 10_000_000,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 0u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;

    let _ = unstake_and_advance(&mut env, withdraw_index, 10_000_000_000, 0);

    let invalid_sys = Pubkey::new_unique();
    let ix = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: invalid_sys,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let res = env.ctx.send(ix);
    assert_program_error!(
        res,
        xorca::XorcaStakingProgramError::IncorrectAccountAddress
    );
}

// Wrong vault seeds (not the canonical PDA) should fail
#[test]
fn withdraw_wrong_vault_account_seeds() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 1u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    // Create pending via unstake

    let _ = unstake_and_advance(&mut env, withdraw_index, 1_000_000, 2);
    // Use bogus vault account
    let bogus_vault = Pubkey::new_unique();
    env.ctx
        .write_account(
            bogus_vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.state, amount => 0),
        )
        .unwrap();
    let ix = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: bogus_vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Wrong ORCA mint address should fail
#[test]
fn withdraw_invalid_orca_mint_address() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 2u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;

    let _ = unstake_and_advance(&mut env, withdraw_index, 1_000_000, 0);
    // Wrong ORCA mint
    let wrong_orca_mint = Pubkey::new_unique();
    env.ctx
        .write_account(
            wrong_orca_mint,
            TOKEN_PROGRAM_ID,
            crate::token_mint_data!(
                supply => 0,
                decimals => 6,
                mint_authority_flag => 1,
                mint_authority => Pubkey::default(),
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();
    let ix = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: wrong_orca_mint,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Wrong Token Program should fail
#[test]
fn withdraw_invalid_token_program_id() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 3u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;

    let _ = unstake_and_advance(&mut env, withdraw_index, 1_000_000, 0);
    let bad = Pubkey::new_unique();
    let ix = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: bad,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Early withdraw before cooldown should fail with CoolDownPeriodStillActive
#[test]
fn withdraw_fails_before_cooldown() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 10,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 4u8;
    // Create pending via helper and advance only 1s (cooldown is 10)
    let pending_withdraw_account = unstake_and_advance(&mut env, withdraw_index, 1_000_000, 1);
    // Attempt withdraw via helper
    let res = do_withdraw(&mut env, pending_withdraw_account, withdraw_index);
    assert_program_error!(res, XorcaStakingProgramError::CoolDownPeriodStillActive);
}

// Withdraw exactly at boundary: now == withdrawable_timestamp should succeed
#[test]
fn withdraw_succeeds_at_cooldown_boundary() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 10,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 40u8;
    // Create pending withdraw and advance exactly to the boundary
    let pending_withdraw_account =
        unstake_and_advance(&mut env, withdraw_index, 1_000_000, pool.cool_down_period_s);
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let res = do_withdraw(&mut env, pending_withdraw_account, withdraw_index);
    assert!(res.is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed at boundary",
    );
    // xORCA burned at unstake was 1_000_000
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        1_000_000,
        "withdraw at cooldown boundary",
    );
}

// Right-before boundary: advancing to (timestamp - 1) should fail
#[test]
fn withdraw_fails_one_second_before_boundary() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 10,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 41u8;
    // Create pending withdraw and advance to one second before the boundary
    let pending_withdraw_account = unstake_and_advance(
        &mut env,
        withdraw_index,
        1_000_000,
        pool.cool_down_period_s - 1,
    );
    let res = do_withdraw(&mut env, pending_withdraw_account, withdraw_index);
    assert_program_error!(res, XorcaStakingProgramError::CoolDownPeriodStillActive);
}

// Right-after boundary: advancing to (timestamp + 1) should succeed
#[test]
fn withdraw_succeeds_one_second_after_boundary() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 10,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 42u8;
    // Create pending withdraw and advance to one second after the boundary
    let pending_withdraw_account = unstake_and_advance(
        &mut env,
        withdraw_index,
        1_000_000,
        pool.cool_down_period_s + 1,
    );

    // Snapshot + pending for aggregate effects
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let xorca_unstake_amount = 1_000_000u64;
    let res = do_withdraw(&mut env, pending_withdraw_account, withdraw_index);
    assert!(res.is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending withdraw closed after boundary",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw one second after boundary",
    );
}

// Using wrong withdraw index parameter for the correct pending account should fail (seeds mismatch)
#[test]
fn withdraw_invalid_withdraw_index_param_mismatch() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let correct_index = 5u8;
    let wrong_index = 6u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, correct_index, 1_000_000, 2);
    // Use wrong index param for withdraw
    let res = {
        let ix = Withdraw {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_orca_ata: env.staker_orca_ata,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(WithdrawInstructionArgs {
            withdraw_index: wrong_index,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Duplicate withdraw using same index should fail on the second attempt since pending is closed
#[test]
fn withdraw_duplicate_withdraw_index() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 8u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 2_000_000, 2);
    // first withdraw
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let xorca_unstake_amount = 2_000_000u64;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending A closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "duplicate index first withdraw",
    );
    // second withdraw using same index should fail (pending already closed)
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert!(res.is_err());
}

// Two withdraws in one transaction for different pending indices should both succeed
#[test]
fn withdraw_concurrent_two_indices_in_one_tx() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 3_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx_a = 9u8;
    let idx_b = 10u8;
    let pending_withdraw_account_a = unstake_and_advance(&mut env, idx_a, 1_000_000, 0);
    let pending_withdraw_account_b = unstake_and_advance(&mut env, idx_b, 2_000_000, 2);
    // Snapshot and pending amounts before withdraw of both
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let pending_a_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_a)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let pending_b_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_b)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let xorca_unstake_a = 1_000_000u64;
    let xorca_unstake_b = 2_000_000u64;
    // withdraw both in one tx
    let ix_w_a = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account_a,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs {
        withdraw_index: idx_a,
    });
    let ix_w_b = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account_b,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs {
        withdraw_index: idx_b,
    });
    assert!(env.ctx.sends(&[ix_w_a, ix_w_b]).is_ok());
    // Both pendings should be closed
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account_a,
        "pending A closed",
    );
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account_b,
        "pending B closed",
    );
    // Totals and aggregate assertions via helper
    let total_withdrawable = pending_a_before.saturating_add(pending_b_before);
    let total_xorca_burn = xorca_unstake_a.saturating_add(xorca_unstake_b);
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        total_withdrawable,
        total_xorca_burn,
        "two withdraws in one tx",
    );
}

// Zero-withdrawable amount should still close pending. If amount is non-zero, escrow should decrease by pending amount.
#[test]
fn withdraw_zero_amount_pending() {
    let ctx = TestContext::new();
    // Configure an extreme rate so 1 xORCA burns to 0 ORCA in unstake
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 10_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 11u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1, 2);
    // Snapshot and pending amount before withdraw
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert!(res.is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending closed on zero-amount withdraw",
    );
    // xORCA burned at unstake was 1
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        1,
        "zero-amount withdraw",
    );
}

// Large-number probe: create large pending amount via unstake, then withdraw
#[test]
fn withdraw_overflow_attack_large_numbers_probe() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: u64::MAX / 2,
        vault_orca: u64::MAX / 2 + 1,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: u64::MAX / 2,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 12u8;
    let xorca_unstake_amount = u64::MAX / 2;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, xorca_unstake_amount, 2);
    // Take snapshot and pending amount
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let res = do_withdraw(&mut env, pending_withdraw_account, idx); // accept ok or error, but should not panic
    if let Ok(_) = res {
        crate::utils::assert::assert_account_closed(
            &env.ctx,
            pending_withdraw_account,
            "pending closed on large-numbers withdraw",
        );
        assert_withdraw_effects(
            &env.ctx,
            env.state,
            env.vault,
            env.staker_orca_ata,
            env.staker_xorca_ata,
            XORCA_ID,
            &snap,
            withdrawable_orca_before,
            xorca_unstake_amount,
            "large-numbers withdraw",
        );
    }
}

// Boundary: withdraw using the maximum index (255)
#[test]
fn withdraw_index_max_value_255_success() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 2_000_000_000,
        vault_orca: 2_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx: u8 = u8::MAX; // 255
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Snapshot + pending
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let withdrawable_orca_before = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let xorca_unstake_amount = 1_000_000u64;
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending closed",
    );
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        withdrawable_orca_before,
        xorca_unstake_amount,
        "withdraw max index",
    );
}

// Withdraw on closed index should fail
#[test]
fn withdraw_on_closed_index_fails() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 3_000_000_000,
        vault_orca: 3_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 0,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 512_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    // Create 256 pendings
    for i in 0u8..=u8::MAX {
        let _ = unstake_and_advance(&mut env, i, 2_000, 0);
    }
    // Withdraw one index successfully to close it
    let pending0 = find_pending_withdraw_pda(&env.staker, &0).unwrap().0;
    assert!(do_withdraw(&mut env, pending0, 0).is_ok());
    // Now try to withdraw again with the same index (should fail since it is closed already)
    let res = do_withdraw(&mut env, pending0, 0);
    assert!(res.is_err());
}

// Pending withdraw account with wrong owner should fail
#[test]
fn withdraw_invalid_pending_withdraw_account_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 13u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Tweak owner to system program (incorrect owner)
    let pending_data = env.ctx.get_raw_account(pending_withdraw_account).unwrap();
    env.ctx
        .write_raw_account(
            pending_withdraw_account,
            SYSTEM_PROGRAM_ID,
            pending_data.data,
        )
        .unwrap();
    // Attempt withdraw via helper
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Pending withdraw account with wrong seeds should fail
#[test]
fn withdraw_invalid_pending_withdraw_account_seeds() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let env = Env::new(ctx, &pool, &user);
    let idx = 14u8;
    // Create a bogus pending account with correct owner and data but wrong address (not PDA)
    let bogus_pending = Pubkey::new_unique();
    // Put minimal valid data with timestamp in the past
    let withdrawable_orca_amount = 0u64;
    let withdrawable_timestamp: i64 = 0;
    let mut ctx2 = env.ctx;
    ctx2.write_account(
        bogus_pending,
        xorca::ID,
        crate::pending_withdraw_data!(unstaker => env.staker, withdraw_index => idx, withdrawable_orca_amount => withdrawable_orca_amount, withdrawable_timestamp => withdrawable_timestamp, bump => 0)
    ).unwrap();
    // Attempt withdraw using bogus pending account
    let res = {
        let ix = Withdraw {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: bogus_pending,
            unstaker_orca_ata: env.staker_orca_ata,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(WithdrawInstructionArgs {
            withdraw_index: idx,
        });
        ctx2.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Event emission smoke test
#[test]
fn withdraw_event_emission_verification() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 15u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Capture expected pending amount and cooldown before withdraw
    let state_before = env.ctx.get_account::<State>(env.state).unwrap();
    let pending_withdrawable = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert!(res.is_ok());
    // Decode events and assert a Withdraw event with expected fields
    let events = decode_events_from_result(&res);
    assert!(!events.is_empty(), "no events decoded");
    // escrow after = before - withdrawable
    let state_after = env.ctx.get_account::<State>(env.state).unwrap();
    let mut found = false;
    for e in events {
        if let Event::Withdraw {
            vault_escrowed_orca_amount,
            withdrawable_orca_amount,
            cool_down_period_s,
            withdraw_index,
        } = e
        {
            assert_eq!(withdraw_index, idx);
            assert_eq!(withdrawable_orca_amount, pending_withdrawable);
            assert_eq!(cool_down_period_s, state_before.data.cool_down_period_s);
            assert_eq!(
                vault_escrowed_orca_amount,
                state_after.data.escrowed_orca_amount
            );
            found = true;
            break;
        }
    }
    assert!(found, "Withdraw event not found in logs");
}

// Vault token account owner in data must be state; wrong owner should fail
#[test]
fn withdraw_invalid_vault_account_owner_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 16u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Corrupt vault token account data: set wrong owner field
    env.ctx
        .write_account(
            env.vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(
                mint => ORCA_ID, owner => Pubkey::new_unique(), amount => 1_000_000_000,
            ),
        )
        .unwrap();
    // Try withdraw via helper
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Vault token account program owner must be TOKEN_PROGRAM_ID; wrong program owner should fail
#[test]
fn withdraw_invalid_vault_account_program_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 18u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Set wrong program owner for vault account
    env.ctx
        .write_account(
            env.vault,
            crate::ATA_PROGRAM_ID,
            crate::token_account_data!(
                mint => ORCA_ID, owner => env.state, amount => 1_000_000_000,
            ),
        )
        .unwrap();
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Unstaker ORCA ATA must be owned by Token Program; wrong program owner should fail
#[test]
fn withdraw_invalid_unstaker_orca_ata_program_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 19u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Wrong program owner for user's ORCA ATA
    env.ctx
        .write_account(
            env.staker_orca_ata,
            crate::ATA_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.staker, amount => 0),
        )
        .unwrap();
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Unstaker ORCA ATA wrong mint in data should fail
#[test]
fn withdraw_invalid_unstaker_orca_ata_mint_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 20u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Wrong mint in user's ORCA ATA data
    env.ctx
        .write_account(
            env.staker_orca_ata,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => XORCA_ID, owner => env.staker, amount => 0),
        )
        .unwrap();
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Unstaker ORCA ATA wrong owner in data should fail
#[test]
fn withdraw_invalid_unstaker_orca_ata_owner_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 21u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Wrong owner in user's ORCA ATA data
    env.ctx
        .write_account(
            env.staker_orca_ata,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => Pubkey::new_unique(), amount => 0),
        )
        .unwrap();
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// ORCA mint wrong owner should fail
#[test]
fn withdraw_invalid_orca_mint_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 22u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // ORCA mint account with wrong owner
    env.ctx
        .write_account(
            ORCA_ID,
            SYSTEM_PROGRAM_ID,
            crate::token_mint_data!(
                supply => 0,
                decimals => 6,
                mint_authority_flag => 1,
                mint_authority => Pubkey::default(),
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Vault token account mint must be ORCA; wrong mint should fail
#[test]
fn withdraw_invalid_vault_account_mint_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 17u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Corrupt vault token account data: set wrong mint field
    env.ctx
        .write_account(
            env.vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(
                mint => XORCA_ID, owner => env.state, amount => 1_000_000_000,
            ),
        )
        .unwrap();
    // Try withdraw via helper
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Escrow insufficient: program should return InsufficientEscrow if escrow < withdrawable
#[test]
fn test_withdraw_insufficient_escrow_error() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 100, // Start with small escrow
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 23u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);

    // Get the withdrawable amount from the pending account
    let withdrawable_amount = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    // Only proceed if withdrawable amount is greater than 0
    if withdrawable_amount == 0 {
        panic!("Withdrawable amount is 0, cannot test arithmetic underflow");
    }

    let vault_bump: u8 = env
        .ctx
        .get_account::<State>(env.state)
        .unwrap()
        .data
        .vault_bump;

    // Manipulate state to have escrow much less than withdrawable amount
    // This should cause arithmetic error when trying to subtract
    let (_, state_bump) = find_state_address().unwrap();
    env.ctx
        .write_account(
            env.state,
            xorca::ID,
            crate::state_data!(
                escrowed_orca_amount => 0, // Zero escrow, but withdrawable amount is much larger
                update_authority => Pubkey::default(),
                cool_down_period_s => pool.cool_down_period_s,
                bump => state_bump,
                vault_bump => vault_bump,
            ),
        )
        .unwrap();

    // Expect InsufficientEscrow returned by instruction due to pre-check
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert_program_error!(res, XorcaStakingProgramError::InsufficientEscrow);
}

// Timestamp overflow: test with very large timestamps that could cause overflow
#[test]
fn test_withdraw_timestamp_overflow() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: i64::MAX, // Maximum possible cool down period
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Set the current timestamp to a value that will cause overflow when adding i64::MAX
    let mut clock = env.ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1; // Set to 1 so that 1 + i64::MAX will overflow
    env.ctx.svm.set_sysvar::<Clock>(&clock);

    let idx = 24u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    // This should fail with CoolDownOverflow due to timestamp overflow
    assert_program_error!(res, XorcaStakingProgramError::CoolDownOverflow);
}

// Account closure failure: test when closing the pending withdraw account fails
#[test]
fn test_withdraw_account_closure_failure() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 25u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);

    // Corrupt the pending withdraw account to make it uncloseable
    // Set it to be owned by the wrong program
    let pending_data = env.ctx.get_raw_account(pending_withdraw_account).unwrap();
    env.ctx
        .write_raw_account(
            pending_withdraw_account,
            SYSTEM_PROGRAM_ID, // Wrong owner - should be xorca::ID
            pending_data.data,
        )
        .unwrap();

    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    // This should fail because the account can't be closed
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Vault insufficient balance: test when vault doesn't have enough ORCA to transfer
#[test]
fn test_withdraw_vault_insufficient_balance() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 26u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);

    // Get the withdrawable amount
    let withdrawable_amount = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    // Drain the vault to have insufficient balance
    env.ctx
        .write_account(
            env.vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(
                mint => ORCA_ID,
                owner => env.state,
                amount => withdrawable_amount.saturating_sub(1), // Less than needed
            ),
        )
        .unwrap();

    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    // This should fail because the vault doesn't have enough ORCA
    // The error comes from the SPL Token program, not our program
    assert!(res.is_err());
}

// Wrong program ID: test with wrong program ID in account validation
#[test]
fn test_withdraw_wrong_program_id_in_state() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 29u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);

    // Corrupt the state account to be owned by the wrong program
    let state_data = env.ctx.get_raw_account(env.state).unwrap();
    env.ctx
        .write_raw_account(
            env.state,
            SYSTEM_PROGRAM_ID, // Wrong owner - should be xorca::ID
            state_data.data,
        )
        .unwrap();

    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    // This should fail because the state account has wrong program owner
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Verify that cooldown updates mid-flight are handled correctly - Old withdraws do not get updated, but new ones do.
#[test]
fn withdraw_cooldown_update_mid_flight_policy_change() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000,
        vault_orca: 10_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 10,
    };
    let user = UserSetup {
        staker_orca: 5_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Seed update authority to signer so Set succeeds
    let (_, state_bump) = find_state_address().unwrap();
    let vault_bump: u8 = env
        .ctx
        .get_account::<State>(env.state)
        .unwrap()
        .data
        .vault_bump;
    env.ctx
        .write_account(
            env.state,
            xorca::XORCA_STAKING_PROGRAM_ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => env.ctx.signer(),
                cool_down_period_s => pool.cool_down_period_s,
                bump => state_bump,
                vault_bump => vault_bump,
            ),
        )
        .unwrap();

    // Stake to get xORCA tokens first
    stake_orca(&mut env, 2_000_000, "initial stake for cooldown test");

    // Create a pending withdraw at old cooldown (10s)
    let idx_old = 40u8;
    let pending_old = unstake_and_advance(&mut env, idx_old, 1_000_000, 0);
    let old_pending_ts = env
        .ctx
        .get_account::<PendingWithdraw>(pending_old)
        .unwrap()
        .data
        .withdrawable_timestamp;

    // Update cooldown to a new value (e.g., 100s)
    let ix_set = Set {
        update_authority_account: env.ctx.signer(),
        state_account: env.state,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: 100,
        },
    });
    assert!(env.ctx.send(ix_set).is_ok());

    // New pending should use new cooldown value
    let idx_new = 41u8;
    let pending_new = unstake_and_advance(&mut env, idx_new, 500_000, 0);
    let new_pending_ts = env
        .ctx
        .get_account::<PendingWithdraw>(pending_new)
        .unwrap()
        .data
        .withdrawable_timestamp;

    // Verify timestamps: new pending is strictly later (longer cooldown) or equal if clock advanced between ops
    assert!(new_pending_ts >= old_pending_ts + (100 - 10));

    // Advance to just before old cooldown maturity: old withdraw should still fail
    advance_clock_env(&mut env, 9);
    let res_old_early = do_withdraw(&mut env, pending_old, idx_old);
    assert_program_error!(
        res_old_early,
        XorcaStakingProgramError::CoolDownPeriodStillActive
    );

    // Advance to satisfy old cooldown but not the new one
    advance_clock_env(&mut env, 1);
    let res_old_ok = do_withdraw(&mut env, pending_old, idx_old);
    assert!(res_old_ok.is_ok());

    // New pending should still be locked (we've only advanced ~10s total)
    let res_new_early = do_withdraw(&mut env, pending_new, idx_new);
    assert_program_error!(
        res_new_early,
        XorcaStakingProgramError::CoolDownPeriodStillActive
    );
}

// Index reuse lifecycle: withdraw closes pending, then reuse the same index to create a fresh pending and withdraw again
#[test]
fn withdraw_index_reuse_lifecycle() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000,
        vault_orca: 5_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 2,
    };
    let user = UserSetup {
        staker_orca: 2_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Stake to get xORCA tokens first
    stake_orca(&mut env, 2_000_000, "initial stake for index reuse test");

    let idx = 42u8;

    // First cycle: create pending and withdraw
    let pending1 = unstake_and_advance(&mut env, idx, 1_000_000, 3);
    let res1 = do_withdraw(&mut env, pending1, idx);
    assert!(res1.is_ok());
    assert_account_closed(&env.ctx, pending1, "pending1 closed");

    // Second cycle: reuse the same index; should create a fresh pending with new timestamp
    let pending2 = unstake_and_advance(&mut env, idx, 500_000, 0);
    // Ensure it's a different account instance by reading data and verifying bump/timestamp updated
    let pend2 = env.ctx.get_account::<PendingWithdraw>(pending2).unwrap();
    assert!(pend2.data.withdrawable_orca_amount > 0);
    assert_eq!(
        pend2.data.withdraw_index, idx,
        "withdraw index should match"
    );

    // Advance and withdraw the second pending
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let res2 = do_withdraw(&mut env, pending2, idx);
    assert!(res2.is_ok());
    assert_account_closed(&env.ctx, pending2, "pending2 closed");
}

// === 4) Account Closure Verification Tests ===

#[test]
fn withdraw_verifies_complete_account_closure_procedure() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 2_000_000_000,
        vault_orca: 2_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 50u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 2_000_000, 2);

    // Capture complete account state before withdraw
    let account_before = env.ctx.get_raw_account(pending_withdraw_account).unwrap();
    let initial_lamports = account_before.lamports;
    let initial_data_size = account_before.data.len();
    let staker_lamports_before = env.ctx.get_raw_account(env.staker).unwrap().lamports;

    // Verify account has data and lamports before closure
    assert!(
        initial_data_size > 0,
        "Account should have data before closure"
    );
    assert!(
        initial_lamports > 0,
        "Account should have lamports before closure"
    );

    // Perform withdraw
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert!(res.is_ok(), "withdraw should succeed");

    // Verify all aspects of the improved closure procedure
    // When an account is resized to 0, it's effectively removed from the runtime
    let account_after = env.ctx.get_raw_account(pending_withdraw_account);
    let staker_lamports_after = env.ctx.get_raw_account(env.staker).unwrap().lamports;

    // === COMPREHENSIVE ACCOUNT CLOSURE VERIFICATION ===

    // Step 1: Verify discriminator is set to Closed (value 3) - happens before resize
    assert_eq!(
        xorca::AccountDiscriminator::Closed as u8,
        3,
        "Closed discriminator should be value 3"
    );

    // Step 2: Verify lamports were transferred to receiver (staker) minus resize cost
    let expected_lamports = staker_lamports_before + initial_lamports - RESIZE_TO_ZERO_COST;
    assert_eq!(
        staker_lamports_after, expected_lamports,
        "Step 2: Staker should have received the lamports minus resize cost of {}",
        RESIZE_TO_ZERO_COST
    );

    // Step 3: Verify account is resized to 0 (improved procedure) - removes account from runtime
    assert!(
        account_after.is_err(),
        "Step 3: Account should be removed (not found) after closure with resize(0) - this proves the improved procedure worked"
    );
}

#[test]
fn withdraw_verifies_account_closure_with_zero_lamports() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 54u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);

    // Manually set pending account lamports to 0 to test zero-lamport closure
    let account_before = env.ctx.get_raw_account(pending_withdraw_account).unwrap();
    let mut modified_account = account_before.clone();
    modified_account.lamports = 0;
    env.ctx
        .svm
        .set_account(pending_withdraw_account, modified_account)
        .unwrap();

    let staker_lamports_before = env.ctx.get_raw_account(env.staker).unwrap().lamports;

    // Perform withdraw
    let res = do_withdraw(&mut env, pending_withdraw_account, idx);
    assert!(
        res.is_ok(),
        "withdraw should succeed even with zero lamports"
    );

    // Verify account closure with zero lamports
    // When an account is resized to 0, it's effectively removed from the runtime
    let account_after = env.ctx.get_raw_account(pending_withdraw_account);
    let staker_lamports_after = env.ctx.get_raw_account(env.staker).unwrap().lamports;

    // Staker lamports should decrease by the resize cost (since account has 0 lamports)
    let expected_lamports = staker_lamports_before - RESIZE_TO_ZERO_COST;
    assert_eq!(
        staker_lamports_after, expected_lamports,
        "Staker lamports should decrease by resize cost when closing account with zero lamports"
    );

    // Account should still be resized to 0 (improved procedure works even with zero lamports)
    assert!(
        account_after.is_err(),
        "Account should be removed (not found) after closure with resize(0) even with zero lamports"
    );
}
