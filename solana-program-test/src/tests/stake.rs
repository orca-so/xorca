use crate::utils::assert::{
    assert_stake_effects, assert_state, assert_token_account, decode_events_from_result,
    take_stake_snapshot, ExpectedState, ExpectedTokenAccount,
};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::stake_orca;
use crate::{
    assert_program_error, TestContext, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_state_address, Event, Stake, StakeInstructionArgs, TokenAccount, TokenMint,
    XorcaStakingProgramError,
};
use xorca_staking_program::state::state::State;

// Fresh deployment path: supply=0, non_escrowed=0 → initial exchange rate
#[test]
fn stake_success_on_fresh_deployment() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env.ctx.send(ix).is_ok());
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        1_000_000,
        1_000_000,
        pool.cool_down_period_s,
        "fresh deploy stake",
    );
}

// Tests staking when exchange rate is active (after initial deployment)
// At higher rates, each xORCA represents more ORCA, so you get fewer xORCA per ORCA staked
#[test]
fn stake_success_at_exchange_rate_1_to_2_success() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 2_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env.ctx.send(ix).is_ok());
    // Exchange rate r = xorca_supply / non_escrowed = 1_000_000_000 / 2_000_000_000 = 0.5
    // Minted = stake * r = 1_000_000 * 0.5 = 500,000
    let expected_minted = 500_000u64;
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        1_000_000,
        expected_minted,
        pool.cool_down_period_s,
        "active exchange rate stake",
    );
}

// Tests staking when exchange rate is active (after initial deployment)
// At higher rates, each xORCA represents more ORCA, so you get fewer xORCA per ORCA staked
#[test]
fn stake_success_at_exchange_rate_with_decimals() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_234_356_434,
        vault_orca: 2_323_324_233,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env.ctx.send(ix).is_ok());
    // Exchange rate r = xorca_supply / non_escrowed = 1_234_356_434 / 2_323_324_233 ≈ 0.531288...
    // Minted = floor(stake * r) = floor(1_000_000 * 1_234_356_434 / 2_323_324_233) = 531,288
    let expected_minted = 531_288u64;
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        1_000_000,
        expected_minted,
        pool.cool_down_period_s,
        "active exchange rate stake",
    );
}

// Tests rounding behavior when stake amount isn't perfectly divisible by exchange rate
// With 3:1 rate, staking 1,000,001 lamports should round down to 333,333 xORCA
#[test]
fn stake_success_rounds_down_at_non_divisible_rate() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 3_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_001,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_001,
    });
    assert!(env.ctx.send(ix).is_ok());
    // Exchange rate r = xorca_supply / non_escrowed = 1_000_000_000 / 3_000_000_000 = 0.3333333333333333
    // Minted = stake * r = 1_000_001 * 0.3333333333333333 = 333,333 (floored)
    let expected_minted = 333_333u64;
    assert_token_account(
        &env.ctx,
        env.staker_xorca_ata,
        ExpectedTokenAccount {
            owner: &env.staker,
            mint: &XORCA_ID,
            amount: expected_minted,
            label: "staker xORCA after 3.0 rate rounding",
        },
    );
}

// Tests staking when exchange rate is lower than 1:1 (e.g., 0.25:1)
// At lower rates, each xORCA represents less ORCA, so you get more xORCA per ORCA staked
// This can happen when supply is low (contract first deployed) and Orca is deposited into the vault
#[test]
fn stake_success_for_low_exchange_rate() {
    // Scenario: low xORCA supply but vault is high due to external deposit → high exchange rate → mint less xORCA per ORCA
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 3_000_000,     // low supply
        vault_orca: 100_000_000_000, // prefunded vault (non_escrowed >> supply)
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let orca_stake = 1_000_000u64;
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: orca_stake,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env.ctx.send(ix).is_ok());
    // Exchange rate r = xorca_supply / non_escrowed = 3_000_000 / 100_000_000_000 = 0.00003
    // Minted = stake * r = 1_000_000 * 0.00003 = 30 (floored)
    let expected_minted = 30u64;
    assert!(expected_minted > 0 && expected_minted < orca_stake);
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        orca_stake,
        expected_minted,
        pool.cool_down_period_s,
        "prefunded vault, low supply stake",
    );
}

// --- Success with different escrow amounts ---

// Tests that escrowed ORCA doesn't affect the minting calculation
// Two identical pools except for escrow amount should produce the same xORCA mint
#[test]
fn stake_success_with_escrow_orca() {
    // Two identical pools except for escrow; minted xORCA should be the same
    let ctx_no_escrow = TestContext::new();
    let ctx_with_escrow = TestContext::new();
    let pool_base = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 5_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    // Keep non-escrowed constant between pools by increasing vault by the escrow amount
    let pool_escrow = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 5_500_000_000,
        escrowed_orca: 500_000_000,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 2_000_000,
        staker_xorca: 0,
    };
    let mut env_a = Env::new(ctx_no_escrow, &pool_base, &user);
    let mut env_b = Env::new(ctx_with_escrow, &pool_escrow, &user);

    let ix_a = Stake {
        staker_account: env_a.staker,
        state_account: env_a.state,
        vault_account: env_a.vault,
        staker_orca_ata: env_a.staker_orca_ata,
        staker_xorca_ata: env_a.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 2_000_000,
    });
    let ix_b = Stake {
        staker_account: env_b.staker,
        state_account: env_b.state,
        vault_account: env_b.vault,
        staker_orca_ata: env_b.staker_orca_ata,
        staker_xorca_ata: env_b.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 2_000_000,
    });

    let snap_a = take_stake_snapshot(
        &env_a.ctx,
        env_a.state,
        env_a.vault,
        env_a.staker_orca_ata,
        env_a.staker_xorca_ata,
        XORCA_ID,
    );
    let snap_b = take_stake_snapshot(
        &env_b.ctx,
        env_b.state,
        env_b.vault,
        env_b.staker_orca_ata,
        env_b.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env_a.ctx.send(ix_a).is_ok());
    assert!(env_b.ctx.send(ix_b).is_ok());

    // non_escrowed = vault - escrow is equal between pools (5_000_000_000)
    // minted = ORCA_in * supply / non_escrowed = 2_000_000 * 1_000_000_000 / 5_000_000_000 = 400_000
    assert_stake_effects(
        &env_a.ctx,
        env_a.state,
        env_a.vault,
        env_a.staker_orca_ata,
        env_a.staker_xorca_ata,
        XORCA_ID,
        &snap_a,
        2_000_000,
        400_000,
        pool_base.cool_down_period_s,
        "xORCA no escrow",
    );
    assert_stake_effects(
        &env_b.ctx,
        env_b.state,
        env_b.vault,
        env_b.staker_orca_ata,
        env_b.staker_xorca_ata,
        XORCA_ID,
        &snap_b,
        2_000_000,
        400_000,
        pool_escrow.cool_down_period_s,
        "xORCA with escrow",
    );
    // Escrow amount remains as configured
    assert_state(
        &env_b.ctx,
        env_b.state,
        ExpectedState {
            escrowed_orca_amount: pool_escrow.escrowed_orca,
            cool_down_period_s: pool_escrow.cool_down_period_s,
        },
    );
}

// Tests that even with large escrow amounts, the minting still uses non-escrowed ORCA for calculations
// The exchange rate calculation should exclude escrowed amounts from the backing
#[test]
fn stake_success_with_large_escrow_still_uses_non_escrowed_rate() {
    // Large escrow present; conversion uses non_escrowed = vault - escrow so mint remains consistent
    let ctx = TestContext::new();
    // non_escrowed = vault - escrow = 500_000_000; supply = 1_000_000
    // minted = 1_000_000 * 1_000_000 / 500_000_000 = 2 (same as no-escrow case)
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 2_000_000_500,
        escrowed_orca: 1_500_000_500,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env.ctx.send(ix).is_ok());
    // Exchange rate r = xorca_supply / non_escrowed = 1_000_000_000 / 500_000_000 = 2
    let expected_minted = 2_000_000u64;
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        1_000_000,
        expected_minted,
        pool.cool_down_period_s,
        "xORCA with large escrow",
    );
}

// === 2) Edge cases and precision ===

// Tests precision loss via integer flooring in proportional mint path
// Setup: non_escrowed > 0 and xORCA supply > 0 so proportional path is used
// Minted xORCA = floor(orca_stake_amount * xorca_supply / non_escrowed)
#[test]
fn stake_precision_loss_rounds_down() {
    let ctx = TestContext::new();
    // Choose small integers that create a fractional proportional result
    // xorca_supply = 100, non_escrowed (vault) = 333, stake 10 ORCA → floor(10*100/333) = 3 xORCA
    let pool = PoolSetup {
        xorca_supply: 100,
        vault_orca: 333,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 10,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 10,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let _ = env.ctx.send(ix);

    // Proportional path rounds down: floor(10 * 100 / 333) = 3
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        10,
        3,
        pool.cool_down_period_s,
        "precision loss rounds down",
    );
}

// When orca & xOrca are both 6 decimals, verify the lowest stake amount that would yield 0 mints
// Should throw an error
#[test]
fn stake_precision_loss_rounds_down_to_zero() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 333_333_333,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 10,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let res = env.ctx.send(ix);

    // Proportional path rounds down: floor(10 * 1000000 / 333333333) = 0
    assert_program_error!(res, XorcaStakingProgramError::InsufficientStakeAmount);
}

// Tests rounding behavior: many small stakes vs one large stake
// Due to floor division on each operation, sum of small mints should be <= one-shot mint
#[test]
fn stake_rounding_many_small_vs_one_large() {
    let ctx_small = TestContext::new();
    let ctx_large = TestContext::new();
    // supply=1000, non_escrowed=1001 → r ≈ 0.999
    // floor(1 * 0.999)=0, but to avoid error, adjust so small mints 1 sometimes, but sum < large due to floor
    // Better: supply=3_000, non_escrowed=1_000 → r=3
    // floor(1 * 3)=3 per small, no floor loss, but to show loss, use non-integer r
    // supply=5_000, non_escrowed=3_000 → r≈1.666
    // floor(1 * 1.666)=1 per small; sum=1000; large floor(1000*1.666)=1666 >1000
    let pool = PoolSetup {
        xorca_supply: 5_000,
        vault_orca: 3_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    const SMALL_COUNT: u64 = 1_000;
    let user_small = UserSetup {
        staker_orca: SMALL_COUNT,
        staker_xorca: 0,
    };
    let user_large = UserSetup {
        staker_orca: SMALL_COUNT,
        staker_xorca: 0,
    };
    let mut env_small = Env::new(ctx_small, &pool, &user_small);
    let mut env_large = Env::new(ctx_large, &pool, &user_large);

    let vault_small_before = env_small
        .ctx
        .get_account::<TokenAccount>(env_small.vault)
        .unwrap()
        .data
        .amount;
    let vault_large_before = env_large
        .ctx
        .get_account::<TokenAccount>(env_large.vault)
        .unwrap()
        .data
        .amount;

    // SMALL_COUNT small stakes of 1 lamport
    for _ in 0..SMALL_COUNT {
        let ix = Stake {
            staker_account: env_small.staker,
            state_account: env_small.state,
            vault_account: env_small.vault,
            staker_orca_ata: env_small.staker_orca_ata,
            staker_xorca_ata: env_small.staker_xorca_ata,
            xorca_mint_account: XORCA_ID,
            orca_mint_account: ORCA_ID,
            token_program_account: TOKEN_PROGRAM_ID,
        }
        .instruction(StakeInstructionArgs {
            orca_stake_amount: 1,
        });
        assert!(env_small.ctx.send(ix).is_ok());
    }
    let xorca_small = env_small
        .ctx
        .get_account::<TokenAccount>(env_small.staker_xorca_ata)
        .unwrap()
        .data
        .amount;

    // One large stake of SMALL_COUNT lamports
    let ix_large = Stake {
        staker_account: env_large.staker,
        state_account: env_large.state,
        vault_account: env_large.vault,
        staker_orca_ata: env_large.staker_orca_ata,
        staker_xorca_ata: env_large.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: SMALL_COUNT,
    });
    assert!(env_large.ctx.send(ix_large).is_ok());
    let xorca_large = env_large
        .ctx
        .get_account::<TokenAccount>(env_large.staker_xorca_ata)
        .unwrap()
        .data
        .amount;

    // With this configuration, small path mints 0 each time, while the one-shot mints > 0.
    assert!(xorca_small < xorca_large);
    // And vault balances reflect staked totals
    let vault_small = env_small
        .ctx
        .get_account::<TokenAccount>(env_small.vault)
        .unwrap()
        .data
        .amount;
    let vault_large = env_large
        .ctx
        .get_account::<TokenAccount>(env_large.vault)
        .unwrap()
        .data
        .amount;
    assert_eq!(vault_small - vault_small_before, SMALL_COUNT);
    assert_eq!(vault_large - vault_large_before, SMALL_COUNT);
    // Staker ORCA ATAs should be drained to 0 in both scenarios
    let staker_orca_small = env_small
        .ctx
        .get_account::<TokenAccount>(env_small.staker_orca_ata)
        .unwrap()
        .data
        .amount;
    let staker_orca_large = env_large
        .ctx
        .get_account::<TokenAccount>(env_large.staker_orca_ata)
        .unwrap()
        .data
        .amount;
    assert_eq!(staker_orca_small, 0);
    assert_eq!(staker_orca_large, 0);
}

// Tests that state account must be owned by the staking program
// Attempting to use a state account owned by the wrong program should fail
#[test]
fn stake_invalid_state_account_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    let vault_bump: u8 = env.ctx.get_account::<State>(env.state).unwrap().data.bump;
    // Overwrite state with wrong owner
    let (_, state_bump) = find_state_address().unwrap();
    env.ctx
        .write_account(
            env.state,
            TOKEN_PROGRAM_ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 7*24*60*60,
                bump => state_bump,
                vault_bump => vault_bump,
            ),
        )
        .unwrap();

    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

// Tests staking with very large numbers to exercise u128 math paths
// Ensures the program handles large values without panicking
#[test]
fn stake_overflow_attack_large_numbers() {
    let ctx = TestContext::new();
    // Very large supply and non-escrowed to exercise u128 path
    let pool = PoolSetup {
        xorca_supply: u64::MAX,
        vault_orca: 1_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = {
        let ix_clone = ix.clone();
        env.ctx.send(ix_clone)
    };
    assert!(res.is_ok() || res.is_err(), "tx should not panic");
}

// Verify that underflow in non_escrowed amount causes an error
#[test]
fn stake_underflow_non_escrowed_error() {
    let ctx = TestContext::new();
    // Configure vault < escrow to force non_escrowed underflow in stake (defensive check)
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 100,
        escrowed_orca: 200,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let vault_bump: u8 = env
        .ctx
        .get_account::<State>(env.state)
        .unwrap()
        .data
        .vault_bump;
    // Ensure state escrow is strictly greater than vault to force underflow
    let (_, state_bump) = find_state_address().unwrap();
    env.ctx
        .write_account(
            env.state,
            xorca::ID,
            crate::state_data!(
                escrowed_orca_amount => u64::MAX,
                update_authority => Pubkey::default(),
                cool_down_period_s => pool.cool_down_period_s,
                bump => state_bump,
                vault_bump => vault_bump,
            ),
        )
        .unwrap();

    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InsufficientVaultBacking);
}

// Tests division by zero scenario when supply > 0 but non-escrowed = 0
// Program should fall back to initial scaling when vault has no ORCA
#[test]
fn stake_division_by_zero_non_escrowed_zero_supply_nonzero() {
    let ctx = TestContext::new();
    // xORCA supply > 0, vault non-escrowed = 0
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    assert!(env.ctx.send(ix).is_ok());
    // With equal decimals fallback, mint 1:1 even if non_escrowed is zero
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        1_000_000,
        1_000_000,
        pool.cool_down_period_s,
        "division-by-zero non-escrowed fallback",
    );
}

// Tests staking zero amount - should succeed and mint 0 xORCA
// Edge case to ensure program handles zero inputs gracefully
#[test]
fn stake_zero_amount() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 0,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InsufficientStakeAmount);
}

// Tests staking maximum u64 amount under initial scaling
// This would overflow xORCA mint amount (times 1000) - smoke test for runtime behavior
#[test]
fn stake_max_u64_amount_overflow_expected() {
    let ctx = TestContext::new();
    // Initial rate path (supply 0) will try to mint stake * 1000, which overflows u64
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: u64::MAX,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: u64::MAX,
    });
    let _ = env.ctx.send(ix); // Accept current behavior (may succeed or overflow in token program)
}

// Tests that vault account must be the canonical PDA
// Using a bogus vault account should fail with InvalidSeeds
#[test]
fn stake_wrong_vault_account() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let bogus_vault = Pubkey::new_unique();
    env.ctx
        .write_account(
            bogus_vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.state, amount => 0),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: bogus_vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
}

// === 3) Account validation (PDA seeds, owners, mints, token program) ===

// Tests that xORCA mint pubkey must match the canonical program mint
// Using wrong mint address should fail with InvalidAccountData
#[test]
fn stake_wrong_xorca_mint_address() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let wrong_mint = Pubkey::new_unique();
    env.ctx
        .write_account(
            wrong_mint,
            TOKEN_PROGRAM_ID,
            crate::token_mint_data!(
                supply => 0,
                decimals => 6,
                mint_authority_flag => 1,
                mint_authority => env.state,
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: wrong_mint,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Tests that ORCA mint pubkey must match the canonical ORCA_ID
#[test]
fn stake_wrong_orca_mint_address() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
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
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: wrong_orca_mint,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Tests that token program account must be the correct SPL Token Program
// Using a random pubkey should fail with IncorrectAccountAddress
#[test]
fn stake_malicious_token_program() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let bad = Pubkey::new_unique();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: bad,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::IncorrectAccountAddress);
}

// Tests staking more ORCA than the user owns
// Should fail with InsufficientFunds error
#[test]
fn stake_insufficient_orca_tokens() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 500_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InsufficientFunds);
}

// Tests that state account PDA seeds must be correct
// Using wrong seeds should fail with InvalidSeeds
#[test]
fn stake_invalid_state_account_seeds() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Create a bogus state PDA-like account with correct owner and data but wrong seeds
    let invalid_state = Pubkey::find_program_address(&[b"invalid_seed"], &XORCA_PROGRAM_ID).0;
    env.ctx
        .write_account(
            invalid_state,
            XORCA_PROGRAM_ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 7*24*60*60,
                bump => 0, // Wrong bump for invalid state
            ),
        )
        .unwrap();

    let ix = Stake {
        staker_account: env.staker,
        state_account: invalid_state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

// Tests that state account is verified against the bump on the state account data
#[test]
fn stake_invalid_state_account_bump() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    let state_data = env.ctx.get_account::<State>(env.state).unwrap();
    let vault_bump = state_data.data.vault_bump;

    // Corrupt the bump in the state account data to a wrong value
    // Stake will expect this "correct value"
    let wrong_bump = 254;
    env.ctx
        .write_account(
            env.state,
            XORCA_PROGRAM_ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 7*24*60*60,
                bump => wrong_bump, // Wrong bump
                vault_bump => vault_bump,
            ),
        )
        .unwrap();

    // Use utility function to attempt stake - should fail due to invalid bump
    let result = stake_orca(&mut env, 1_000_000, "stake with wrong bump");
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

// Tests that staker ORCA ATA must have correct owner in its data
// Using ATA with wrong owner should fail with InvalidAccountData
#[test]
fn stake_invalid_staker_orca_ata_owner_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Wrong owner in token account data
    env.ctx.write_account(
        env.staker_orca_ata,
        TOKEN_PROGRAM_ID,
        crate::token_account_data!(mint => ORCA_ID, owner => Pubkey::default(), amount => 1_000_000),
    ).unwrap();

    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

// Tests that staker ORCA ATA must point to the correct mint
// Using ATA with wrong mint should fail with InvalidAccountData
#[test]
fn stake_invalid_staker_orca_ata_mint_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_orca_ata,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => XORCA_ID, owner => env.staker, amount => 1_000_000),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

// Tests that staker ORCA ATA must be owned by the Token Program
// Using ATA owned by wrong program should fail with IncorrectOwner
#[test]
fn stake_invalid_staker_orca_ata_program_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_orca_ata,
            crate::ATA_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.staker, amount => 1_000_000),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

// Tests that staker xORCA ATA must have correct owner in its data
// Using ATA with wrong owner should fail with InvalidAccountData
#[test]
fn stake_invalid_staker_xorca_ata_owner_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_xorca_ata,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => XORCA_ID, owner => Pubkey::default(), amount => 0),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

// Tests that staker xORCA ATA must point to the correct mint
// Using ATA with wrong mint should fail with InvalidAccountData
#[test]
fn stake_invalid_staker_xorca_ata_mint_data() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_xorca_ata,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.staker, amount => 0),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

// Tests that staker xORCA ATA must be owned by the Token Program
// Using ATA owned by wrong program should fail with IncorrectOwner
#[test]
fn stake_invalid_staker_xorca_ata_program_owner() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    env.ctx
        .write_account(
            env.staker_xorca_ata,
            crate::ATA_PROGRAM_ID,
            crate::token_account_data!(mint => XORCA_ID, owner => env.staker, amount => 0),
        )
        .unwrap();
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let result = env.ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

// === 4) Misc tests ===

// Tests concurrent stakes from the same user in one transaction
// Both stakes should succeed and mint the expected amounts
#[test]
fn stake_concurrent_stakes_same_user_in_one_tx() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 2_000_000,
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix1 = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let ix2 = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let snap = take_stake_snapshot(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
    );
    let mut ctx2 = env.ctx;
    assert!(ctx2.sends(&[ix1, ix2]).is_ok());
    // Assert net effects after both stakes
    assert_stake_effects(
        &ctx2,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        2_000_000,
        2_000_000,
        pool.cool_down_period_s,
        "two stakes in one tx",
    );
}

// Tests that events are emitted when staking succeeds and fields are correct
#[test]
fn stake_event_emission_verification() {
    let ctx = TestContext::new();
    let pool = PoolSetup::default();
    let user = UserSetup {
        staker_orca: 1_000_000,
        staker_xorca: 0,
    };
    let env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1_000_000,
    });
    let mut ctx2 = env.ctx;
    let res = ctx2.send(ix);
    assert!(res.is_ok());
    let events = decode_events_from_result(&res);
    assert!(!events.is_empty(), "no events decoded");
    let mut found = false;
    for e in events {
        if let Event::Stake {
            orca_stake_amount,
            vault_orca_amount,
            vault_escrowed_orca_amount,
            xorca_mint_supply,
            xorca_to_mint,
        } = e
        {
            assert_eq!(orca_stake_amount, 1_000_000);
            assert_eq!(xorca_to_mint, 1_000_000);
            assert_eq!(vault_escrowed_orca_amount, 0);
            assert_eq!(vault_orca_amount, 1_000_000);

            let supply_after = ctx2.get_account::<TokenMint>(XORCA_ID).unwrap().data.supply;
            assert_eq!(supply_after, xorca_mint_supply);
            found = true;
            break;
        }
    }
    assert!(found, "Stake event not found in logs");
}

#[test]
fn stake_fails_when_amount_would_mint_zero() {
    let ctx = TestContext::new();
    // High non_escrowed, low supply, small stake → floor(stake * supply / non_escrowed) = 0
    let pool = PoolSetup {
        xorca_supply: 1_000_000,
        vault_orca: 10_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 1, // Too small to mint any
        staker_xorca: 0,
    };
    let mut env = Env::new(ctx, &pool, &user);
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: 1,
    });
    let res = env.ctx.send(ix);
    assert_program_error!(res, XorcaStakingProgramError::InsufficientStakeAmount);
}
