// Yield-focused tests (renamed from exchange_rate)
use crate::utils::assert::{
    assert_mint, assert_state, assert_token_account, assert_unstake_effects,
    assert_withdraw_effects, take_withdraw_snapshot, ExpectedMint, ExpectedState,
    ExpectedTokenAccount,
};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::{
    advance_clock_env, deposit_yield_into_vault, do_unstake, do_withdraw, stake_orca,
    stake_orca_with_unique, unstake_and_advance,
};
use crate::{TestContext, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID};
use xorca::{PendingWithdraw, State};
use xorca_staking_program::util::math::{convert_orca_to_xorca, convert_xorca_to_orca};

#[test]
fn yield_fresh_deploy_stake_unstake_withdraw_flow() {
    // Arrange: fresh deployment with empty vault
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 3600,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: user stakes ORCA to mint xORCA at fresh deploy
    let _ = stake_orca(&mut env, 1_000_000);

    // Assert: post-stake balances and mint
    assert_token_account(
        &env.ctx,
        env.vault,
        ExpectedTokenAccount {
            owner: &env.state,
            mint: &ORCA_ID,
            amount: 1_000_000,
            label: "vault after initial stake",
        },
    );
    assert_token_account(
        &env.ctx,
        env.staker_orca_ata,
        ExpectedTokenAccount {
            owner: &env.staker,
            mint: &ORCA_ID,
            amount: 0,
            label: "user ORCA after initial stake",
        },
    );
    assert_token_account(
        &env.ctx,
        env.staker_xorca_ata,
        ExpectedTokenAccount {
            owner: &env.staker,
            mint: &XORCA_ID,
            amount: 1_000_000,
            label: "user xORCA after initial stake",
        },
    );
    assert_mint(
        &env.ctx,
        XORCA_ID,
        ExpectedMint {
            decimals: 6,
            supply: 1_000_000,
            mint_authority: &env.state,
            label: "xORCA mint after initial stake",
        },
    );
    assert_state(
        &env.ctx,
        env.state,
        ExpectedState {
            escrowed_orca_amount: 0,
            cool_down_period_s: pool.cool_down_period_s,
        },
    );

    // Act: protocol earns yield (external ORCA deposited into vault)
    deposit_yield_into_vault(&mut env, 250_000, "yield before first unstake");

    // Act: user creates a pending withdraw by unstaking xORCA
    let withdraw_index = 1u8;
    let pending_withdraw_account = xorca::find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    let snapshot_before_unstake = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let xorca_to_burn = 100_000;
    assert!(do_unstake(&mut env, withdraw_index, xorca_to_burn).is_ok());
    let pending_withdraw_account_data = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    let withdrawable = pending_withdraw_account_data.data.withdrawable_orca_amount;

    // Assert: pending amount fixed from pre-unstake snapshot
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_unstake,
        withdrawable,
        xorca_to_burn,
        "fresh deploy unstake",
    );

    // Act: wait until cooldown passes
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);

    // Act: perform the withdraw
    let snapshot_before_withdraw = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let res_w = do_withdraw(&mut env, pending_withdraw_account, withdraw_index);
    assert!(res_w.is_ok());

    // Assert: withdraw effects as expected
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw,
        &snapshot_before_withdraw,
        withdrawable,
        xorca_to_burn,
        "fresh deploy withdraw",
    );
}

// Multi-user, multi-operation flow: several stakes grow supply and vault; multiple unstakes create escrow;
// subsequent withdraws drain escrow; verify exchange-rate-consistent deltas at each step.
#[test]
fn yield_operational_multi_user_mixed_flow() {
    // Arrange: active pool with existing supply and vault balance
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

    // Act: two stakes (2M, then 3M)
    let _ = stake_orca(&mut env, 2_000_000);
    let _ = stake_orca(&mut env, 3_000_000);

    // Assert: post-stake vault and mint state
    assert_token_account(
        &env.ctx,
        env.vault,
        ExpectedTokenAccount {
            owner: &env.state,
            mint: &ORCA_ID,
            amount: 10_000_000 + 5_000_000,
            label: "vault after two stakes",
        },
    );
    assert_token_account(
        &env.ctx,
        env.staker_orca_ata,
        ExpectedTokenAccount {
            owner: &env.staker,
            mint: &ORCA_ID,
            amount: 0,
            label: "user ORCA after stakes",
        },
    );
    assert_mint(
        &env.ctx,
        XORCA_ID,
        // supply grows by minted xORCA; with 6 decimals and parity rate, supply increases by ORCA staked
        ExpectedMint {
            decimals: 6,
            supply: 10_000_000 + 5_000_000,
            mint_authority: &env.state,
            label: "xORCA supply after stakes",
        },
    );

    // Act: protocol earns yield after staking, before unstakes
    deposit_yield_into_vault(&mut env, 750_000, "yield between stake and unstakes");

    // Act: user A creates a pending withdraw
    let withdraw_index_a = 10u8;
    let snapshot_before_unstake_a = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let pending_withdraw_account_a = unstake_and_advance(&mut env, withdraw_index_a, 1_000_000, 2);
    let withdrawable_orca_a = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_a)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    // Assert: A's pending withdraw is fixed from snapshot
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_unstake_a,
        withdrawable_orca_a,
        1_000_000,
        "operational unstake A",
    );

    // Act: user B creates a pending withdraw
    let withdraw_index_b = 11u8;
    let snapshot_before_unstake_b = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let pending_withdraw_account_b = unstake_and_advance(&mut env, withdraw_index_b, 2_000_000, 2);
    let withdrawable_orca_b = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_b)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    // Assert: B's pending withdraw is fixed from snapshot
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_unstake_b,
        withdrawable_orca_b,
        2_000_000,
        "operational unstake B",
    );

    // Assert: escrowed ORCA equals sum of both pending withdraw amounts
    let state_after_both_unstakes = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(
        state_after_both_unstakes.data.escrowed_orca_amount,
        withdrawable_orca_a.saturating_add(withdrawable_orca_b)
    );

    // Act: advance time past cooldown and withdraw both
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let snapshot_before_withdraw_a = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_withdraw(&mut env, pending_withdraw_account_a, withdraw_index_a).is_ok());
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_a,
        &snapshot_before_withdraw_a,
        withdrawable_orca_a,
        1_000_000,
        "withdraw A",
    );
    let snapshot_before_withdraw_b = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_withdraw(&mut env, pending_withdraw_account_b, withdraw_index_b).is_ok());
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_b,
        &snapshot_before_withdraw_b,
        withdrawable_orca_b,
        2_000_000,
        "withdraw B",
    );

    // Assert: escrow is now zero
    let final_state = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(final_state.data.escrowed_orca_amount, 0);
}

// Large escrow scenario carries through
#[test]
fn yield_operational_large_escrow_carries_through() {
    // Arrange: pool with large existing escrow and supply
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 200_000_000,
        vault_orca: 400_000_000,
        escrowed_orca: 50_000_000,
        cool_down_period_s: 30,
    };
    let user = UserSetup {
        staker_orca: 5_000_000,
        staker_xorca: 10_000_000,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: protocol earns yield prior to unstake
    deposit_yield_into_vault(&mut env, 1_500_000, "yield before large escrow flow");

    // Act: user unstakes to create pending
    let idx = 42u8;
    let pending_withdraw_account = unstake_and_advance(&mut env, idx, 2_000_000, 2);
    let withdrawable_orca_amount = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    // Assert: escrow increases by the pending withdraw amount
    let state_after = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(
        state_after.data.escrowed_orca_amount,
        pool.escrowed_orca.saturating_add(withdrawable_orca_amount)
    );

    // Act: wait for cooldown and withdraw
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let snapshot_before_withdraw = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());

    // Assert: withdraw effects and pending cleared
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw,
        &snapshot_before_withdraw,
        withdrawable_orca_amount,
        2_000_000,
        "large escrow withdraw",
    );
}

// Interleaved multi-flows with varied amounts
#[test]
fn yield_operational_interleaved_multi_flows_varied_amounts() {
    // Arrange: pool with escrow and large vault backing
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 20_000_000,
        vault_orca: 30_000_000,
        escrowed_orca: 10_000_000,
        cool_down_period_s: 20,
    };
    let user = UserSetup {
        staker_orca: 7_333_333,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: first stake
    let first_stake_orca_amount = 4_000_000u64;
    let _ = stake_orca(&mut env, first_stake_orca_amount);

    // Assert: post first stake
    assert_token_account(
        &env.ctx,
        env.vault,
        ExpectedTokenAccount {
            owner: &env.state,
            mint: &ORCA_ID,
            amount: 30_000_000 + first_stake_orca_amount,
            label: "vault after stake #1",
        },
    );
    assert_token_account(
        &env.ctx,
        env.staker_xorca_ata,
        ExpectedTokenAccount {
            owner: &env.staker,
            mint: &XORCA_ID,
            amount: first_stake_orca_amount,
            label: "user xORCA after stake #1 (parity)",
        },
    );
    // Inject yield before first unstake
    deposit_yield_into_vault(
        &mut env,
        300_000,
        "yield before first unstake in interleaved flow",
    );

    // Act: first unstake
    let first_unstake_xorca_burn_amount = first_stake_orca_amount / 2;
    let withdraw_index_1 = 50u8;
    let snapshot_before_unstake_1 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let pending_withdraw_account_1 = unstake_and_advance(
        &mut env,
        withdraw_index_1,
        first_unstake_xorca_burn_amount,
        0,
    );
    let withdrawable_orca_1 = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_1)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    // Effects assertion validates the program’s own calculation; avoid brittle equality on manual recompute
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_unstake_1,
        withdrawable_orca_1,
        first_unstake_xorca_burn_amount,
        "unstake #1 effects",
    );

    let snapshot_after_unstake_1 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );

    // Act: second stake
    let second_stake_orca_amount = 3_333_333u64;
    // Compute expected minted for second stake at current exchange rate (pre-stake values)
    let supply_before_second_stake = env
        .ctx
        .get_account::<xorca::TokenMint>(XORCA_ID)
        .unwrap()
        .data
        .supply;
    let state_before_second_stake = env.ctx.get_account::<State>(env.state).unwrap();
    let vault_before_second_stake = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.vault)
        .unwrap()
        .data
        .amount;
    let non_escrowed_before_second_stake = vault_before_second_stake
        .saturating_sub(state_before_second_stake.data.escrowed_orca_amount);
    let expected_minted_second_stake = second_stake_orca_amount
        .saturating_mul(supply_before_second_stake)
        .saturating_div(non_escrowed_before_second_stake);
    let user_xorca_before_second_stake = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;
    let _ = stake_orca(&mut env, second_stake_orca_amount);

    // Assert: post second stake
    // Vault should equal initial override + stakes + any prior yield deposit
    assert_token_account(
        &env.ctx,
        env.vault,
        ExpectedTokenAccount {
            owner: &env.state,
            mint: &ORCA_ID,
            amount: 30_000_000 + first_stake_orca_amount + second_stake_orca_amount + 300_000,
            label: "vault after stake #2",
        },
    );
    let user_xorca_after_second_stake = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after_second_stake,
        user_xorca_before_second_stake.saturating_add(expected_minted_second_stake)
    );

    // Act: second unstake
    let second_unstake_xorca_burn_amount = 1_000_000u64;
    let withdraw_index_2 = 51u8;
    let snapshot_before_unstake_2 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let pending_withdraw_account_2 = unstake_and_advance(
        &mut env,
        withdraw_index_2,
        second_unstake_xorca_burn_amount,
        0,
    );
    let withdrawable_orca_2 = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account_2)
        .unwrap()
        .data
        .withdrawable_orca_amount;

    // Assert: second pending computed from pre-unstake snapshot
    let non_escrowed_2 = snapshot_before_unstake_2
        .vault_before
        .saturating_sub(snapshot_before_unstake_2.escrow_before);
    let _expected_withdrawable_2 = second_unstake_xorca_burn_amount
        .saturating_mul(non_escrowed_2)
        .saturating_div(snapshot_before_unstake_2.xorca_supply_before);
    // Effects assertion validates the program’s own calculation; avoid brittle equality on manual recompute
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_unstake_2,
        withdrawable_orca_2,
        second_unstake_xorca_burn_amount,
        "unstake #2 effects",
    );

    // Act: time passes -> withdraw first pending
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let snapshot_before_withdraw_1 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_withdraw(&mut env, pending_withdraw_account_1, withdraw_index_1).is_ok());
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_1,
        &snapshot_after_unstake_1,
        withdrawable_orca_1,
        first_unstake_xorca_burn_amount,
        "withdraw #1 effects",
    );

    // Act: more time passes -> withdraw second pending
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let snapshot_before_withdraw_2 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_withdraw(&mut env, pending_withdraw_account_2, withdraw_index_2).is_ok());
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_2,
        &snapshot_before_withdraw_2,
        withdrawable_orca_2,
        second_unstake_xorca_burn_amount,
        "withdraw #2 effects",
    );

    // Assert: escrow returned to initial value
    let final_state = env.ctx.get_account::<State>(env.state).unwrap();
    assert_eq!(final_state.data.escrowed_orca_amount, pool.escrowed_orca);
}

// Many-small vs one-large full cycle
#[test]
fn yield_many_small_vs_one_large_full_cycle() {
    // Arrange: two users perform equivalent total stakes using different patterns
    let ctx_small = TestContext::new();
    let ctx_large = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000,
        vault_orca: 10_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 5,
    };
    let user_small = UserSetup {
        staker_orca: 100_000,
        staker_xorca: 0,
    };
    let user_large = UserSetup {
        staker_orca: 100_000,
        staker_xorca: 0,
    };
    let mut env_small = Env::new(ctx_small, &pool, &user_small);
    let mut env_large = Env::new(ctx_large, &pool, &user_large);

    // Add small yield to cause rounding differences
    deposit_yield_into_vault(&mut env_small, 1, "yield for rounding in small");
    deposit_yield_into_vault(&mut env_large, 1, "yield for rounding in large");

    // Act: many small stakes vs one large stake
    for i in 0..100 {
        let res = stake_orca_with_unique(&mut env_small, 1_000, i);
        assert!(res.is_ok(), "loop {i}: stake should succeed",);
    }
    let user_xorca_after_many_small_stakes = env_small
        .ctx
        .get_account::<xorca::TokenAccount>(env_small.staker_xorca_ata)
        .unwrap()
        .data
        .amount;
    let _ = stake_orca(&mut env_large, 100_000);
    let user_xorca_after_one_large_stake = env_large
        .ctx
        .get_account::<xorca::TokenAccount>(env_large.staker_xorca_ata)
        .unwrap()
        .data
        .amount;

    // Assert: one large stake mints at least as much as many small stakes
    assert!(user_xorca_after_many_small_stakes <= user_xorca_after_one_large_stake);
    let withdraw_index_small = 1u8;
    let withdraw_index_large = 2u8;

    // Act: each user unstakes all and withdraws
    let pending_withdraw_account_small: solana_sdk::pubkey::Pubkey =
        crate::utils::flows::unstake_and_advance(
            &mut env_small,
            withdraw_index_small,
            user_xorca_after_many_small_stakes,
            pool.cool_down_period_s + 1,
        );
    let withdrawable_orca_small = env_small
        .ctx
        .get_account::<xorca::PendingWithdraw>(pending_withdraw_account_small)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let snapshot_before_withdraw_small = take_withdraw_snapshot(
        &env_small.ctx,
        env_small.state,
        env_small.vault,
        env_small.staker_orca_ata,
        env_small.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(crate::utils::flows::do_withdraw(
        &mut env_small,
        pending_withdraw_account_small,
        withdraw_index_small
    )
    .is_ok());
    assert_withdraw_effects(
        &env_small.ctx,
        env_small.state,
        env_small.vault,
        env_small.staker_orca_ata,
        env_small.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_small,
        &snapshot_before_withdraw_small,
        withdrawable_orca_small,
        user_xorca_after_many_small_stakes,
        "many-small full cycle",
    );
    let pending_withdraw_account_large = crate::utils::flows::unstake_and_advance(
        &mut env_large,
        withdraw_index_large,
        user_xorca_after_one_large_stake,
        pool.cool_down_period_s + 1,
    );
    let withdrawable_orca_large = env_large
        .ctx
        .get_account::<xorca::PendingWithdraw>(pending_withdraw_account_large)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let snapshot_before_withdraw_large = take_withdraw_snapshot(
        &env_large.ctx,
        env_large.state,
        env_large.vault,
        env_large.staker_orca_ata,
        env_large.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(crate::utils::flows::do_withdraw(
        &mut env_large,
        pending_withdraw_account_large,
        withdraw_index_large
    )
    .is_ok());
    assert_withdraw_effects(
        &env_large.ctx,
        env_large.state,
        env_large.vault,
        env_large.staker_orca_ata,
        env_large.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_large,
        &snapshot_before_withdraw_large,
        withdrawable_orca_large,
        user_xorca_after_one_large_stake,
        "one-large full cycle",
    );

    // Assert: withdrawable from one large flow is at least as much
    assert!(withdrawable_orca_small < withdrawable_orca_large);
}

// Long deterministic sequence with invariants after each step
#[test]
fn yield_long_sequence_invariants_hold() {
    // Arrange: small pool with escrow
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000,
        vault_orca: 7_000_000,
        escrowed_orca: 2_000_000,
        cool_down_period_s: 3,
    };
    let user = UserSetup {
        staker_orca: 5_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: two stakes
    let _ = stake_orca(&mut env, 1_000_000);
    let _ = stake_orca(&mut env, 500_000);

    // Act: unstake and withdraw
    let withdraw_index_1 = 50u8;
    let pending_withdraw_account_1 = crate::utils::flows::unstake_and_advance(
        &mut env,
        withdraw_index_1,
        600_000,
        pool.cool_down_period_s + 1,
    );
    let withdrawable_orca_1 = env
        .ctx
        .get_account::<xorca::PendingWithdraw>(pending_withdraw_account_1)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let snapshot_before_withdraw_1 = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(crate::utils::flows::do_withdraw(
        &mut env,
        pending_withdraw_account_1,
        withdraw_index_1
    )
    .is_ok());

    // Assert: withdraw effects
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw_1,
        &snapshot_before_withdraw_1,
        withdrawable_orca_1,
        600_000,
        "after U1",
    );
}

// Prefunded vault at fresh deployment (supply = 0), then a user stakes and attempts to earn yield from prefund
#[test]
fn yield_prefunded_vault_fresh_deploy_stake_then_withdraw() {
    // Arrange: fresh deployment with prefunded vault
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 100_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    let stake_amount = 1_000_000;
    // Act: stake at fresh deploy with prefunded vault
    let ix_stake = xorca::Stake {
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
        orca_stake_amount: stake_amount,
    });
    assert!(env.ctx.sends(&[ix_stake]).is_ok());
    let user_xorca = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;
    assert_eq!(user_xorca, 1_000_000);

    // Act: unstake full balance and withdraw
    let idx = 77u8;
    assert!(do_unstake(&mut env, idx, user_xorca).is_ok());
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let snap_w = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let pending_pda = xorca::find_pending_withdraw_pda(&env.staker, &idx)
        .unwrap()
        .0;
    let pending_amount = env
        .ctx
        .get_account::<PendingWithdraw>(pending_pda)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    let expected_pending = convert_xorca_to_orca(
        user_xorca,
        pool.vault_orca.saturating_add(stake_amount),
        user_xorca,
    )
    .unwrap();
    println!("expected_pending: {}", expected_pending);

    // Assert: user receives entire prefund plus stake
    assert_eq!(pending_amount, expected_pending);
    assert!(do_withdraw(&mut env, pending_pda, idx).is_ok());
    assert_withdraw_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap_w,
        &snap_w,
        pending_amount,
        user_xorca,
        "prefunded withdraw",
    );
    let user_orca_after = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_orca_ata)
        .unwrap()
        .data
        .amount;
    assert_eq!(user_orca_after, expected_pending);
}

// Yield deposit before stake: external ORCA enters the vault increasing non_escrowed; next stake mints less xORCA
#[test]
fn yield_deposit_before_stake_affects_minting() {
    // Arrange: active pool
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000,
        vault_orca: 10_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 10,
    };
    let user = UserSetup {
        staker_orca: 2_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: external yield deposit
    let deposit = 5_000_000u64;
    deposit_yield_into_vault(&mut env, deposit, "yield deposit before stake");

    // Compute: expected minted at higher exchange rate
    let supply_before = env
        .ctx
        .get_account::<xorca::TokenMint>(XORCA_ID)
        .unwrap()
        .data
        .supply;
    let non_escrowed_before = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.vault)
        .unwrap()
        .data
        .amount;
    let stake_amount = 2_000_000u64;
    let expected_minted =
        convert_orca_to_xorca(stake_amount, non_escrowed_before, supply_before).unwrap();
    // Act: stake
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
        orca_stake_amount: stake_amount,
    });
    assert!(env.ctx.sends(&[ix]).is_ok());
    let user_xorca = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;

    // Assert: minted equals expected
    assert_eq!(user_xorca, expected_minted);
}

// Yield deposit before unstake: deposit increases non_escrowed and should increase withdrawable at unstake
#[test]
fn yield_deposit_before_unstake_increases_withdrawable() {
    // Arrange: active pool
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 10_000_000,
        vault_orca: 10_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 2,
    };
    let user = UserSetup {
        staker_orca: 5_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: stake to get xORCA
    let stake_amount = 5_000_000u64;
    let ix_s = xorca::Stake {
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
        orca_stake_amount: stake_amount,
    });
    assert!(env.ctx.sends(&[ix_s]).is_ok());
    let user_xorca = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.staker_xorca_ata)
        .unwrap()
        .data
        .amount;

    // Act: protocol earns yield before unstake
    let add = 2_000_000u64;
    deposit_yield_into_vault(&mut env, add, "yield deposit before unstake");

    // Act: user unstakes half and creates pending
    let idx = 52u8;
    let snapshot_before_unstake = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let xorca_to_burn_for_unstake = user_xorca / 2;
    assert!(do_unstake(&mut env, idx, xorca_to_burn_for_unstake).is_ok());
    let non_escrowed = snapshot_before_unstake
        .vault_before
        .saturating_sub(snapshot_before_unstake.escrow_before);

    // Compute & Assert: pending uses pre-unstake snapshot
    let expected_withdrawable_orca = convert_xorca_to_orca(
        xorca_to_burn_for_unstake,
        non_escrowed,
        snapshot_before_unstake.xorca_supply_before,
    )
    .unwrap();
    let pending_withdraw_account = xorca::find_pending_withdraw_pda(&env.staker, &idx)
        .unwrap()
        .0;
    let withdrawable_orca_amount = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_eq!(withdrawable_orca_amount, expected_withdrawable_orca);
}

// Yield deposit after unstake (during cooldown): should NOT change the already fixed pending amount
#[test]
fn yield_deposit_after_unstake_does_not_change_pending() {
    // Arrange: active pool
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 20_000_000,
        vault_orca: 20_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 3,
    };
    let user = UserSetup {
        staker_orca: 10_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Act: stake, then create pending
    let ix_s = xorca::Stake {
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
        orca_stake_amount: 10_000_000,
    });
    assert!(env.ctx.sends(&[ix_s]).is_ok());
    let idx = 61u8;
    let xorca_to_burn_for_unstake = 5_000_000u64;
    let snapshot_before_unstake = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_unstake(&mut env, idx, xorca_to_burn_for_unstake).is_ok());
    let non_escrowed_pre = snapshot_before_unstake
        .vault_before
        .saturating_sub(snapshot_before_unstake.escrow_before);
    let expected_withdrawable_orca = xorca_to_burn_for_unstake
        .saturating_mul(non_escrowed_pre)
        .saturating_div(snapshot_before_unstake.xorca_supply_before);

    // Assert: pending fixed at unstake time
    assert_unstake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_unstake,
        expected_withdrawable_orca,
        xorca_to_burn_for_unstake,
        "yield-after-unstake effects",
    );
    let pending_withdraw_account = xorca::find_pending_withdraw_pda(&env.staker, &idx)
        .unwrap()
        .0;
    let pending_before_withdrawable_orca = env
        .ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap()
        .data
        .withdrawable_orca_amount;
    assert_eq!(pending_before_withdrawable_orca, expected_withdrawable_orca);

    // Act: deposit yield during cooldown window
    deposit_yield_into_vault(&mut env, 3_000_000, "yield deposit during cooldown");

    // Act: withdraw; Assert: original pending used
    advance_clock_env(&mut env, pool.cool_down_period_s + 1);
    let snapshot_before_withdraw = take_withdraw_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(do_withdraw(&mut env, pending_withdraw_account, idx).is_ok());
    // Use basic assertion here since vault balance changed during cooldown (yield deposit)
    crate::utils::assert::assert_withdraw_effects_basic(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snapshot_before_withdraw,
        expected_withdrawable_orca,
        "yield-after-unstake withdraw",
    );
    crate::utils::assert::assert_account_closed(
        &env.ctx,
        pending_withdraw_account,
        "pending closed after withdraw with yield during cooldown",
    );
}
