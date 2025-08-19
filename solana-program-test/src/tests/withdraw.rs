use crate::utils::assert::{
    assert_withdraw_effects, decode_events_from_result, take_withdraw_snapshot,
};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::{do_withdraw, unstake_and_advance};
use crate::{
    assert_program_error, TestContext, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_pending_withdraw_pda, Event, PendingWithdraw, State, TokenAccount, Withdraw,
    WithdrawInstructionArgs, XorcaStakingProgramError,
};

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

// Probe: extreme large numbers for escrow and withdrawable; expect no panic (may error)
#[test]
#[ignore = "TODO: harden math with checked ops; enable once fixed"]
fn withdraw_extreme_large_values_probe() {
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
    let _ = do_withdraw(&mut env, pending_withdraw_account, idx);
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
    env.ctx.write_account(wrong_orca_mint, TOKEN_PROGRAM_ID, crate::token_mint_data!(supply => 0, decimals => 6, mint_authority_flag => 1, mint_authority => Pubkey::default(), is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default())).unwrap();
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
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
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
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: u64::MAX / 2,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 12u8;
    let xorca_unstake_amount = (u64::MAX / 2).min(10_000_000_000);
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
        crate::pending_withdraw_data!(unstaker => env.staker, withdrawable_orca_amount => withdrawable_orca_amount, withdrawable_timestamp => withdrawable_timestamp)
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

// Overflow/underflow probe: set escrow less than pending amount and attempt withdraw (should panic)
#[test]
#[ignore = "TODO: use checked subtraction for escrow updates; enable when fixed"]
#[should_panic]
fn withdraw_escrow_underflow_attempt() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 100,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 12u8;

    // Create pending via flow and advance past cooldown
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 1_000_000, 2);
    // Manipulate: set state escrow artificially small (zero) while pending amount remains
    env.ctx.write_account(
        env.state,
        xorca::ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(), cool_down_period_s => pool.cool_down_period_s,
        ),
    ).unwrap();
    // Attempt withdraw; expected to panic (ignored until math is hardened)
    let _ = do_withdraw(&mut env, pending_withdraw_account, idx).unwrap();
}
