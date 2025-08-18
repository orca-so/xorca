use crate::utils::assert::{
    assert_pending_withdraw, assert_unstake_effects, decode_events_from_result,
    take_withdraw_snapshot,
};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::do_unstake;
use crate::{
    assert_program_error, TestContext, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use xorca::{find_pending_withdraw_pda, Event, PendingWithdraw, State, XorcaStakingProgramError};

// Happy path: burns xORCA, increases escrow by withdrawable ORCA, and creates a pending withdraw account
#[test]
fn test_unstake_success_at_initial_rate() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 3 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let withdraw_index = 0u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, withdraw_index, 10_000_000_000).is_ok());
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account,
        env.staker,
        1_000_000,
        now,
        "initial rate pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        1_000_000,
        10_000_000_000,
        "initial rate unstake",
    );
}

// Success: high exchange rate (5e9) so non_escrowed=5*supply -> withdrawable ~= 5*xORCA
#[test]
fn test_unstake_succeeds_at_high_exchange_rate() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 5_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx: u8 = 23u8;
    let pending_withdraw_account: Pubkey = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let xorca_burn = 1_000_000u64;
    assert!(do_unstake(&mut env, idx, xorca_burn).is_ok());
    let non_escrowed = snap.vault_before.saturating_sub(snap.escrow_before);
    let expected = xorca_burn
        .saturating_mul(non_escrowed)
        .saturating_div(snap.xorca_supply_before);
    let pend = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account,
        env.staker,
        expected,
        now,
        "high rate pending",
    );
    assert_eq!(pend.data.withdrawable_orca_amount, expected);
    assert!(expected >= 5 * xorca_burn / 1); // lower bound sanity
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        expected,
        xorca_burn,
        "high rate",
    );
}

// Success: low exchange rate (e9=1e5) so non_escrowed=supply/1e4 -> withdrawable ~= xORCA/1e4 (may round down)
#[test]
fn test_unstake_succeeds_at_low_exchange_rate() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 100_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 50_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 24u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let xorca_burn = 50_000_000u64;
    assert!(do_unstake(&mut env, idx, xorca_burn).is_ok());
    let non_escrowed = snap.vault_before.saturating_sub(snap.escrow_before);
    let expected = xorca_burn
        .saturating_mul(non_escrowed)
        .saturating_div(snap.xorca_supply_before);
    let pend = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account,
        env.staker,
        expected,
        now,
        "low rate pending",
    );
    assert_eq!(pend.data.withdrawable_orca_amount, expected);
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        expected,
        xorca_burn,
        "low rate",
    );
}

// Success: existing escrow present; withdrawable uses non-escrowed (vault - escrow) correctly and escrow increases by pending.
#[test]
fn test_unstake_succeeds_with_existing_escrow() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 10_000_000,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 25u8;
    let p = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let xorca_burn = 1_000_000u64;
    assert!(do_unstake(&mut env, idx, xorca_burn).is_ok());
    let non_escrowed = snap.vault_before.saturating_sub(snap.escrow_before);
    let expected = xorca_burn
        .saturating_mul(non_escrowed)
        .saturating_div(snap.xorca_supply_before);
    let pend = env.ctx.get_account::<PendingWithdraw>(p).unwrap();
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        p,
        env.staker,
        expected,
        now,
        "with escrow pending",
    );
    assert_eq!(pend.data.withdrawable_orca_amount, expected);
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        expected,
        xorca_burn,
        "with escrow",
    );
}

// Success: using explicit vault override to set non-escrowed high; expect large withdrawable amount and correct deltas.
#[test]
fn test_unstake_succeeds_with_vault_override_large_non_escrowed() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 500_000_000,
        vault_orca: 5_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 5_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 26u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let xorca_burn = 5_000_000u64;
    assert!(do_unstake(&mut env, idx, xorca_burn).is_ok());
    let non_escrowed = snap.vault_before.saturating_sub(snap.escrow_before);
    let expected = xorca_burn
        .saturating_mul(non_escrowed)
        .saturating_div(snap.xorca_supply_before);
    let pend = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account,
        env.staker,
        expected,
        now,
        "vault override pending",
    );
    assert_eq!(pend.data.withdrawable_orca_amount, expected);
    assert!(expected > 0);
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        expected,
        xorca_burn,
        "vault override",
    );
}

// Success: multiple indices produce multiple pendings with correct amounts and independent timestamps.
#[test]
fn test_unstake_multiple_indices_success() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000,
        vault_orca: 10_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 3_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx_a = 27u8;
    let idx_b = 28u8;
    let pending_withdraw_account_a = find_pending_withdraw_pda(&env.staker, &idx_a).unwrap().0;
    let pending_withdraw_account_b = find_pending_withdraw_pda(&env.staker, &idx_b).unwrap().0;
    // First index
    let snap_a = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx_a, 1_000_000).is_ok());
    let now_a = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let a = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_a)
        .unwrap();
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_a,
        env.staker,
        a.data.withdrawable_orca_amount,
        now_a,
        "multiple indices A pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap_a,
        a.data.withdrawable_orca_amount,
        1_000_000,
        "multiple indices A",
    );

    // Second index
    let snap_b = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx_b, 2_000_000).is_ok());
    let now_b = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let b = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_b)
        .unwrap();
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_b,
        env.staker,
        b.data.withdrawable_orca_amount,
        now_b,
        "multiple indices B pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap_b,
        b.data.withdrawable_orca_amount,
        2_000_000,
        "multiple indices B",
    );

    assert!(a.data.withdrawable_orca_amount > 0 && b.data.withdrawable_orca_amount > 0);
    assert!(
        a.data.withdrawable_timestamp <= b.data.withdrawable_timestamp
            || b.data.withdrawable_timestamp <= a.data.withdrawable_timestamp
    );
}

// Partial unstakes in sequence: two partial burns accumulate escrow and reduce user xORCA by the total.
#[test]
fn test_unstake_partial_two_steps_accumulate_escrow() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 2_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 3_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx1 = 29u8;
    let idx2 = 30u8;
    let pending_withdraw_account_1 = find_pending_withdraw_pda(&env.staker, &idx1).unwrap().0;
    let pending_withdraw_account_2 = find_pending_withdraw_pda(&env.staker, &idx2).unwrap().0;
    let snap1 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx1, 1_000_000).is_ok());
    let now1 = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let pend1 = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_1)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_1,
        env.staker,
        pend1,
        now1,
        "partial step 1 pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap1,
        pend1,
        1_000_000,
        "partial step 1",
    );

    let snap2 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx2, 1_500_000).is_ok());
    let now2 = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let pend2 = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_2)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_2,
        env.staker,
        pend2,
        now2,
        "partial step 2 pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap2,
        pend2,
        1_500_000,
        "partial step 2",
    );

    // Escrow equals initial + both pendings
    let state_after = env
        .ctx
        .get_account::<State>(env.state)
        .unwrap()
        .data
        .escrowed_orca_amount;
    assert_eq!(
        state_after,
        snap1
            .escrow_before
            .saturating_add(pend1)
            .saturating_add(pend2)
    );
    // Verify user xORCA decreased by total burned (no withdraws yet)
    let user_xorca_after = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after,
        snap1
            .user_xorca_before
            .saturating_sub(1_000_000 + 1_500_000)
    );
}

// Partial unstakes with low rate and existing escrow: verify non-escrowed applied and rounding down behavior.
#[test]
fn test_unstake_partial_with_existing_escrow_low_rate() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 5_000_000,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx1 = 31u8;
    let idx2 = 32u8;
    let pending_withdraw_account_1 = find_pending_withdraw_pda(&env.staker, &idx1).unwrap().0;
    let pending_withdraw_account_2 = find_pending_withdraw_pda(&env.staker, &idx2).unwrap().0;
    let snap1 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx1, 500_000).is_ok());

    let vault = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.vault)
        .unwrap();
    let non_escrowed = vault.data.amount.saturating_sub(pool.escrowed_orca);
    let expected1 = 500_000u64
        .saturating_mul(non_escrowed)
        .saturating_div(pool.xorca_supply);
    let expected2 = 500_000u64
        .saturating_mul(non_escrowed)
        .saturating_div(pool.xorca_supply);
    let pend1_acc = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_1)
        .unwrap();
    let pend1 = pend1_acc.data.withdrawable_orca_amount;
    let now1 = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_1,
        env.staker,
        pend1,
        now1,
        "partial low-rate step 1 pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap1,
        pend1,
        500_000,
        "partial low-rate step 1",
    );

    assert!(do_unstake(&mut env, idx2, 500_000).is_ok());
    let pend2_acc = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_2)
        .unwrap();
    let pend2 = pend2_acc.data.withdrawable_orca_amount;
    let now2 = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_2,
        env.staker,
        pend2,
        now2,
        "partial low-rate step 2 pending",
    );
    assert_eq!(pend1, expected1);
    assert_eq!(pend2, expected2);
    // Escrow increased by total pending; compare against initial snapshot
    let final_state = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(
        final_state.data.escrowed_orca_amount,
        snap1
            .escrow_before
            .saturating_add(pend1)
            .saturating_add(pend2)
    );
}

// Partial unstakes leaving dust: burn all but one lamport, then burn last lamport; totals consistent and user xORCA ends at 0.
#[test]
fn test_unstake_partial_all_but_one_then_last() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_001,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx_a = 33u8;
    let idx_b = 34u8;
    let pending_withdraw_account_a = find_pending_withdraw_pda(&env.staker, &idx_a).unwrap().0;
    let pending_withdraw_account_b = find_pending_withdraw_pda(&env.staker, &idx_b).unwrap().0;
    let snap_a = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx_a, 10_000).is_ok());
    let now_a = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let a = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_a)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_a,
        env.staker,
        a,
        now_a,
        "partial dust step A pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap_a,
        a,
        10_000,
        "partial dust step A",
    );

    let snap_b2 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx_b, 1).is_ok());
    let total_burned = 10_001u64;
    let now_b = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let b = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_b)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account_b,
        env.staker,
        b,
        now_b,
        "partial dust step B pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap_b2,
        b,
        1,
        "partial dust step B",
    );
    let state = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(
        state.data.escrowed_orca_amount,
        snap_a.escrow_before.saturating_add(a).saturating_add(b)
    );
    let user_xorca_after = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after,
        snap_a.user_xorca_before.saturating_sub(total_burned)
    );
}

// Duplicate a few invalid-account tests from the original into fixture style

// Invalid: state account owner is wrong program (not staking program).
#[test]
fn test_unstake_invalid_state_account_owner() {
    let ctx = TestContext::new();
    let withdraw_index = 0u8;
    let mut pool = PoolSetup::default();
    pool.xorca_supply = 1_000_000; // avoid arithmetic error path
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Wrong owner for state
    env.ctx.write_account(
        env.state,
        TOKEN_PROGRAM_ID,
        crate::state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    ).unwrap();
    let res = do_unstake(&mut env, withdraw_index, 10_000_000_000);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Invalid: vault token account has wrong mint in its data (not ORCA mint).
#[test]
fn test_unstake_invalid_vault_account_mint_in_data() {
    let ctx = TestContext::new();
    let withdraw_index = 0u8;
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Wrong mint in vault data
    env.ctx.write_account(
        env.vault,
        TOKEN_PROGRAM_ID,
        crate::token_account_data!(
            mint => XORCA_ID,
            owner => env.state,
            amount => 1_000_000_000,
        ),
    ).unwrap();
    let res = do_unstake(&mut env, withdraw_index, 10_000_000_000);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Invalid: token program account is not the SPL Token Program.
#[test]
fn test_unstake_invalid_token_program_id() {
    let ctx = TestContext::new();
    let withdraw_index = 0u8;
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    let invalid_token_program_id = Pubkey::new_unique();
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: invalid_token_program_id,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 10_000_000_000,
            withdraw_index,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Invalid: system program account is not the System Program.
#[test]
fn test_unstake_invalid_system_program_id() {
    let ctx = TestContext::new();
    let withdraw_index = 0u8;
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    let invalid_system_program_id = Pubkey::new_unique();
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: ORCA_ID,
            system_program_account: invalid_system_program_id,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 10_000_000_000,
            withdraw_index,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Zero amount: should succeed, create pending with 0 withdrawable, and xORCA/user/vault/state stay unchanged.
#[test]
fn test_unstake_zero_amount() {
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
    let idx = 1u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx, 0).is_ok());
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account,
        env.staker,
        0,
        now,
        "zero amount pending",
    );
    // Effects: escrow unchanged, xORCA supply/user unchanged, vault/user ORCA unchanged
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        0,
        0,
        "zero amount unstake",
    );
}

// Insufficient funds: unstake more xORCA than the user has should fail with InsufficientFunds.
#[test]
fn test_unstake_insufficient_xorca_tokens() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 2u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::InsufficientFunds);
}

// Wrong vault seeds: use a bogus vault account with correct shape but wrong PDA seeds; expect InvalidSeeds.
#[test]
fn test_unstake_wrong_vault_account_seeds() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 3u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    // Bogus vault account with correct token data
    let bogus_vault = Pubkey::new_unique();
    env.ctx
        .write_account(
            bogus_vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.state, amount => 0),
        )
        .unwrap();
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: bogus_vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 1_000_000,
            withdraw_index: idx,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Invalid xORCA mint address: wrong mint pubkey should be rejected (incorrect account address check).
#[test]
fn test_unstake_invalid_xorca_mint_address() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 4u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let wrong_mint = Pubkey::new_unique();
    env.ctx.write_account(
        wrong_mint,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 9, 
            mint_authority_flag => 1,
            mint_authority => env.state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    ).unwrap();
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: wrong_mint,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 1_000_000,
            withdraw_index: idx,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Invalid ORCA mint address: wrong ORCA mint should be rejected (incorrect account address check).
#[test]
fn test_unstake_invalid_orca_mint_address() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 5u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let wrong_orca = Pubkey::new_unique();
    env.ctx.write_account(
        wrong_orca,
        TOKEN_PROGRAM_ID, crate::token_mint_data!(supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(), is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        ),
    ).unwrap();
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: wrong_orca,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 1_000_000,
            withdraw_index: idx,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Precision loss: unstake 1 lamport of xORCA at high exchange rate so withdrawable rounds to 0.
#[test]
fn test_unstake_precision_loss_attack() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 10_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 6u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx, 1).is_ok());
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let p = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    // With high rate and 1 xORCA, expect small nonzero ORCA if non_escrowed >= supply
    assert!(p.data.withdrawable_orca_amount > 0);
    assert!(p.data.withdrawable_timestamp >= now);
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        p.data.withdrawable_orca_amount,
        1,
        "precision loss unstake",
    );
}

// Rounding: many small unstakes vs one large should satisfy sum(small) <= one large.
#[test]
fn test_unstake_rounding_many_small_vs_one_large() {
    let ctx_small = TestContext::new();
    let ctx_large = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 333_333_333,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user_small = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000,
    };
    let user_large = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000,
    };
    let mut env_small = Env::new(ctx_small, &pool, &user_small);
    let mut env_large = Env::new(ctx_large, &pool, &user_large);
    let mut total_small: u64 = 0;
    for i in 0u8..100u8 {
        let pending_withdraw_account = find_pending_withdraw_pda(&env_small.staker, &i).unwrap().0;
        assert!(do_unstake(&mut env_small, i, 100).is_ok());
        total_small = total_small.saturating_add(
            env_small
                .ctx
                .get_account::<PendingWithdraw>(pending_withdraw_account)
                .unwrap()
                .data
                .withdrawable_orca_amount,
        );
    }
    let pending_withdraw_account_large =
        find_pending_withdraw_pda(&env_large.staker, &0).unwrap().0;
    assert!(do_unstake(&mut env_large, 0, 10_000).is_ok());
    let large = env_large
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_large)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert!(total_small < large);
}

// Concurrency: two unstakes in one tx should both succeed and total escrow equals sum of pending amounts.
#[test]
fn test_unstake_concurrent_unstakes_same_user_in_one_tx() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000,
        vault_orca: 10_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 3_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx_a = 7u8;
    let idx_b = 8u8;
    let pending_withdraw_account_a = find_pending_withdraw_pda(&env.staker, &idx_a).unwrap().0;
    let pending_withdraw_account_b = find_pending_withdraw_pda(&env.staker, &idx_b).unwrap().0;
    let ix_a = xorca::Unstake {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account_a,
        unstaker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::UnstakeInstructionArgs {
        xorca_unstake_amount: 1_000_000,
        withdraw_index: idx_a,
    });
    let ix_b = xorca::Unstake {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account_b,
        unstaker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(xorca::UnstakeInstructionArgs {
        xorca_unstake_amount: 2_000_000,
        withdraw_index: idx_b,
    });
    assert!(env.ctx.sends(&[ix_a, ix_b]).is_ok());
    let a = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_a)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let b = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_b)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let state = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(state.data.escrowed_orca_amount, a.saturating_add(b));
}

// Withdraw index mismatch: PDA derived from index A but instruction uses index B.
#[test]
fn test_unstake_withdraw_index_mismatch() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let correct_index = 9u8;
    let wrong_index = 10u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &correct_index)
        .unwrap()
        .0;
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: pending_withdraw_account,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 1_000_000,
            withdraw_index: wrong_index,
        });
        env.ctx.send(ix)
    };
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// Duplicate withdraw index: using same index twice should fail because pending is no longer a system-owned account.
#[test]
fn test_unstake_duplicate_withdraw_index() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 8_000_000,
        vault_orca: 8_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 2_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 11u8;
    assert!(do_unstake(&mut env, idx, 1_000_000).is_ok());
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Pending withdraw already exists (owned by program) before unstake should fail with IncorrectOwner.
#[test]
fn test_unstake_pending_withdraw_already_exists() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 12u8;
    let p = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    // Pre-create program-owned pending account with minimal valid data
    env.ctx.write_account(
        p,
        xorca::ID,
        crate::pending_withdraw_data!(
            unstaker => env.staker,
            withdrawable_orca_amount => 0, withdrawable_timestamp => 0,
        ),
    ).unwrap();
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Event emission: verify Unstake event fields are emitted with expected values.
#[test]
fn test_unstake_event_emission_verification() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000,
        vault_orca: 5_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 13u8;
    let p = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let res = {
        let ix = xorca::Unstake {
            unstaker_account: env.staker,
            state_account: env.state,
            vault_account: env.vault,
            pending_withdraw_account: p,
            unstaker_xorca_ata: env.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: ORCA_ID,
            system_program_account: SYSTEM_PROGRAM_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(xorca::UnstakeInstructionArgs {
            xorca_unstake_amount: 1_000_000,
            withdraw_index: idx,
        });
        env.ctx.send(ix)
    };
    assert!(res.is_ok());
    let events = decode_events_from_result(&res);
    assert!(!events.is_empty(), "no events decoded");
    let mut found = false;
    for e in events {
        if let Event::Unstake {
            xorca_unstake_amount,
            withdraw_index,
            ..
        } = e
        {
            assert_eq!(xorca_unstake_amount, 1_000_000);
            assert_eq!(withdraw_index, idx);
            found = true;
            break;
        }
    }
    assert!(found, "Unstake event not found");
}

// Division-by-zero path: vault non-escrowed ORCA is zero while xORCA supply > 0; expect ArithmeticError.
// TODO: Improve error message to be more specific
#[test]
fn test_unstake_division_by_zero_non_escrowed_zero() {
    let ctx = TestContext::new();
    // xORCA supply > 0 but vault ORCA (non-escrowed) = 0 via override
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 14u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::ArithmeticError);
}

// Unstake all available xORCA: burn entire balance, escrow increases accordingly, user xORCA goes to 0.
#[test]
fn test_unstake_at_available_amount() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 5_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 16u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx, user.staker_xorca).is_ok());
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let pend = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    assert!(pend.data.withdrawable_orca_amount > 0);
    assert!(pend.data.withdrawable_timestamp >= now);
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        pend.data.withdrawable_orca_amount,
        user.staker_xorca,
        "unstake all",
    );
}

// Cooldown correctness: withdrawable_timestamp equals now + state.cool_down_period_s exactly.
#[test]
fn test_unstake_cool_down_period_calculation_correct() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 42,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 17u8;
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    assert!(do_unstake(&mut env, idx, 1_000_000).is_ok());
    let pend = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    assert_eq!(
        pend.data.withdrawable_timestamp,
        now + pool.cool_down_period_s
    );
}

// Invalid: unstaker xORCA ATA owner in data is wrong (not unstaker) -> InvalidAccountData.
#[test]
fn test_unstake_invalid_unstaker_xorca_ata_owner_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx.write_account(
        env.staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        crate::token_account_data!(
            mint => XORCA_ID, owner => Pubkey::new_unique(), amount => 1_000_000,
        ),
    ).unwrap();
    let idx = 18u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Invalid: unstaker xORCA ATA mint in data is wrong (not xORCA) -> InvalidAccountData.
#[test]
fn test_unstake_invalid_unstaker_xorca_ata_mint_in_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_xorca_ata,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.staker, amount => 1_000_000),
        )
        .unwrap();
    let idx = 19u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Invalid: unstaker xORCA ATA program owner is wrong -> IncorrectOwner.
#[test]
fn test_unstake_invalid_unstaker_xorca_ata_program_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_xorca_ata,
            crate::ATA_PROGRAM_ID,
            crate::token_account_data!(mint => XORCA_ID, owner => env.staker, amount => 1_000_000),
        )
        .unwrap();
    let idx = 20u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}

// Insufficient lamports for pending account creation: reduce unstaker lamports to 0, expect error on account creation.
#[test]
fn test_unstake_account_creation_failure_insufficient_lamports() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    // Drain unstaker SOL to 0
    let mut signer_acc = env.ctx.get_raw_account(env.staker).unwrap();
    signer_acc.lamports = 0;
    env.ctx.svm.set_account(env.staker, signer_acc).unwrap();
    let idx = 21u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert!(
        res.is_err(),
        "should error due to insufficient lamports to create pending"
    );
}

// Supply manipulation: program now validates xORCA mint authority equals state.
#[test]
fn test_unstake_supply_manipulation_attack() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 6_000_000,
        vault_orca: 6_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    // Change xORCA mint authority away from state (still owned by token program)
    let wrong_auth = Pubkey::new_unique();
    env.ctx
        .write_account(
            XORCA_ID,
            TOKEN_PROGRAM_ID,
            crate::token_mint_data!(
                supply => pool.xorca_supply,
                decimals => 9,
                mint_authority_flag => 1,
                mint_authority => wrong_auth,
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();
    let idx = 35u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Freeze authority manipulation: program now validates xORCA freeze authority is null.
#[test]
fn test_unstake_freeze_authority_manipulation() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 7_000_000,
        vault_orca: 7_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 1_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    // Set a freeze authority on the xORCA mint
    let freeze_auth = Pubkey::new_unique();
    env.ctx
        .write_account(
            XORCA_ID,
            TOKEN_PROGRAM_ID,
            crate::token_mint_data!(
                supply => pool.xorca_supply,
                decimals => 9,
                mint_authority_flag => 1,
                mint_authority => env.state,
                is_initialized => true,
                freeze_authority_flag => 1,
                freeze_authority => freeze_auth,
            ),
        )
        .unwrap();
    let idx = 36u8;
    let res = do_unstake(&mut env, idx, 1_000_000);
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Supply = 0: attempting to unstake should fail with ArithmeticError even if vault has non-escrowed ORCA.
#[test]
fn test_unstake_zero_supply_with_nonzero_vault_fails() {
    let ctx = TestContext::new();
    // xORCA supply = 0 to emulate fresh deployment; vault has some ORCA backing
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 500_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 40u8;
    let res = do_unstake(&mut env, idx, 100_000);
    assert_program_error!(res, XorcaStakingProgramError::ArithmeticError);
}

// Supply = 0 and vault non-escrowed = 0: unstake should also fail with ArithmeticError.
#[test]
fn test_unstake_zero_supply_with_zero_vault_fails() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 123_456,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx = 41u8;
    let res = do_unstake(&mut env, idx, 10_000);
    assert_program_error!(res, XorcaStakingProgramError::ArithmeticError);
}

// Boundary: using the maximum withdraw index (255) should work like any other index
#[test]
fn test_unstake_withdraw_index_max_value_255_success() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 10_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let idx: u8 = u8::MAX; // 255
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &idx).unwrap().0;
    let snap = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let burn = 1_000_000u64;
    assert!(do_unstake(&mut env, idx, burn).is_ok());
    let now = env.ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let pend = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_pending_withdraw(
        &env.ctx,
        pending_withdraw_account,
        env.staker,
        pend,
        now,
        "max index pending",
    );
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        pend,
        burn,
        "max index unstake",
    );
}

// Once all 256 indices [0..=255] are used (pendings open), attempting another requires reusing an index and should fail
#[test]
fn test_unstake_withdraw_index_over_limit_behaviour() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 60,
    };
    // Enough xORCA to create 256 small pendings
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 256_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Create pendings for all indices 0..=255
    // Ensure the signer has ample lamports to fund 256 PDA creations
    let mut signer_acc = env.ctx.get_raw_account(env.staker).unwrap();
    signer_acc.lamports = 10_000_000_000; // top-up
    env.ctx.svm.set_account(env.staker, signer_acc).unwrap();

    for i in 0u8..=u8::MAX {
        assert!(
            do_unstake(&mut env, i, 1_000).is_ok(),
            "create pending for index {}",
            i
        );
    }

    // Any additional attempt must reuse an existing index; since the pending exists, it should fail with IncorrectOwner
    let res = do_unstake(&mut env, 0u8, 1_000);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectOwner);
}
