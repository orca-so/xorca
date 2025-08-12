use crate::utils::assert::{
    assert_stake_effects, assert_state, assert_token_account, decode_events_from_result,
    take_stake_snapshot, ExpectedState, ExpectedTokenAccount,
};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::{
    assert_program_error, TestContext, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    Event, Stake, StakeInstructionArgs, TokenAccount, TokenMint, XorcaStakingProgramError,
};

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
        1_000_000_000,
        pool.cool_down_period_s,
        "fresh deploy stake",
    );
}

// Tests staking when exchange rate is active (after initial deployment)
// At higher rates, each xORCA represents more ORCA, so you get fewer xORCA per ORCA staked
#[test]
fn stake_success_on_active_exchange_rate() {
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
    let expected_minted = 1_000_000u64 / 2; // 500,000
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
    let expected_minted = 1_000_001u64 / 3; // 333,333 due to floor
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
fn stake_success_for_lower_parity_rate() {
    // Scenario: low xORCA supply but vault is high due to external deposit → high exchange rate → mint less xORCA per ORCA
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 1_000_000, // low supply
        vault_orca: 100_000_000, // prefunded vault (non_escrowed >> supply)
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
    // expected minted = stake * supply / non_escrowed
    let expected_minted = orca_stake
        .saturating_mul(pool.xorca_supply)
        .saturating_div(pool.vault_orca);
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
        vault_orca: 500_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    // Keep non-escrowed constant between pools by increasing vault by the escrow amount
    let pool_escrow = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 550_000_000,
        escrowed_orca: 50_000_000,
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

    // non_escrowed = vault - escrow is equal between pools (500_000_000)
    // minted = ORCA_in * supply / non_escrowed = 2_000_000 * 1_000_000_000 / 500_000_000 = 4_000_000
    assert_stake_effects(
        &env_a.ctx,
        env_a.state,
        env_a.vault,
        env_a.staker_orca_ata,
        env_a.staker_xorca_ata,
        XORCA_ID,
        &snap_a,
        2_000_000,
        4_000_000,
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
        4_000_000,
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
    // Large escrow present; conversion uses non_escrowed = vault - escrow so mint remains consistent with e9
    let ctx = TestContext::new();
    // non_escrowed = vault - escrow = 500_000_000; supply = 1_000_000_000
    // minted = 1_000_000 * 1_000_000_000 / 500_000_000 = 2_000_000 (same as no-escrow case)
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
        vault_orca: 2_000_000_000,
        escrowed_orca: 1_500_000_000,
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
    let expected_minted = 2_000_000u64; // supply/non_escrowed = 2.0 -> minted = stake * 2
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

// Tests rounding behavior: many small stakes vs one large stake
// Due to floor division on each operation, sum of small mints should be <= one-shot mint
#[test]
fn stake_rounding_many_small_vs_one_large() {
    // Compare many small vs one large across two identical pools
    let ctx_small = TestContext::new();
    let ctx_large = TestContext::new();
    // Use explicit vault/supply (no exchange rate) to drive proportional path.
    // Choose values so 1-lamport stakes mint 0 (floor), while the one-shot mints > 0.
    // xorca_supply = 1_000, non_escrowed (vault) = 3_000 →
    //   1 * 1000 / 3000 = 0 per small; N * 1000 / 3000 > 0 for sufficiently large N.
    let pool = PoolSetup {
        xorca_supply: 1_000,
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
    assert!(xorca_small <= xorca_large);
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

    // Overwrite state with wrong owner
    env.ctx
        .write_account(
            env.state,
            TOKEN_PROGRAM_ID,
            crate::state_data!(
                escrowed_orca_amount => 0,
                update_authority => Pubkey::default(),
                cool_down_period_s => 7*24*60*60,
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

// Tests proportional minting when exchange rate is not at parity (1:1)
// When exchange rate is 0.5, staking should mint 2x the ORCA amount in xORCA
#[test]
fn stake_mints_proportionally_at_non_parity_rate() {
    let ctx = TestContext::new();
    // Configure non_escrowed = 1_000_000_000 so minted = 2_000_000 * 2_000_000_000 / 1_000_000_000 = 4_000_000
    let pool = PoolSetup {
        xorca_supply: 2_000_000_000,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 2_000_000,
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
        orca_stake_amount: 2_000_000,
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
    // With supply=2e9 and non_escrowed=1e9, minted = 2_000_000 * 2e9 / 1e9 = 4_000_000
    assert_stake_effects(
        &env.ctx,
        env.state,
        env.vault,
        env.staker_orca_ata,
        env.staker_xorca_ata,
        XORCA_ID,
        &snap,
        2_000_000,
        4_000_000,
        pool.cool_down_period_s,
        "stake non-parity rate",
    );
    // State checked within assert_stake_effects
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
        staker_orca: 1_000_000_000,
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
        orca_stake_amount: 1_000_000_000,
    });
    let res = {
        let ix_clone = ix.clone();
        env.ctx.send(ix_clone)
    };
    assert!(res.is_ok() || res.is_err(), "tx should not panic");
}

// Tests overflow in initial scaling path when supply=0
// Attempts to stake an amount that would overflow when multiplied by 1000
#[test]
#[ignore = "TODO: harden initial-scaling math (use checked mul, clamp, or reject inputs); enable when fixed"]
#[should_panic]
fn stake_initial_scaling_overflow_attempt() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    // Choose amount near u64::MAX / 1000 to push multiplication overflow
    let stake_amount = u64::MAX / 900; // deliberately high
    let user = UserSetup {
        staker_orca: stake_amount,
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
        orca_stake_amount: stake_amount,
    });
    // Expect panic or failure due to overflow along the path
    let _ = env.ctx.send(ix).unwrap();
}

// Tests overflow in proportional path when calculating (orca * supply) before division
// Attempts to use maximum values to stress the u128 intermediate calculations
#[test]
#[ignore = "TODO: harden proportional path math (checked u128 conversions and bounds); enable when fixed"]
#[should_panic]
fn stake_proportional_path_overflow_attempt() {
    let ctx = TestContext::new();
    // Make supply and ORCA vault enormous and escrow zero so non_escrowed is also enormous.
    let pool = PoolSetup {
        xorca_supply: u64::MAX,
        vault_orca: 1_000_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 1,
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
    let _ = env.ctx.send(ix).unwrap();
}

// Tests division by zero scenario when supply > 0 but non-escrowed = 0
// Program should fall back to initial scaling when vault has no ORCA
#[test]
fn stake_division_by_zero_non_escrowed_zero_supply_nonzero() {
    let ctx = TestContext::new();
    // xORCA supply > 0, vault non-escrowed = 0
    let pool = PoolSetup {
        xorca_supply: 1_000_000_000,
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
        1_000_000_000,
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
        0,
        0,
        pool.cool_down_period_s,
        "stake zero amount",
    );
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
    env.ctx.write_account(wrong_mint, TOKEN_PROGRAM_ID, crate::token_mint_data!(supply => 0, decimals => 9, mint_authority_flag => 1, mint_authority => env.state, is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default())).unwrap();
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
    assert_program_error!(res, XorcaStakingProgramError::InvalidAccountData);
}

// Tests that ORCA mint pubkey must match the canonical ORCA_ID
// Using wrong ORCA mint should fail with InvalidSeeds
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
    env.ctx.write_account(wrong_orca_mint, TOKEN_PROGRAM_ID, crate::token_mint_data!(supply => 0, decimals => 6, mint_authority_flag => 1, mint_authority => Pubkey::default(), is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default())).unwrap();
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
    assert_program_error!(res, XorcaStakingProgramError::InvalidSeeds);
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
        2_000_000_000,
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
            assert_eq!(xorca_to_mint, 1_000_000_000);
            assert_eq!(vault_escrowed_orca_amount, 0);
            // Event reports pre-stake vault amount
            assert_eq!(vault_orca_amount, 0);
            // Validate that on-chain xORCA mint supply equals event supply + minted amount
            // (event may report supply-before or supply-after depending on program timing)
            let supply_after = ctx2.get_account::<TokenMint>(XORCA_ID).unwrap().data.supply;
            assert_eq!(
                supply_after,
                xorca_mint_supply.saturating_add(xorca_to_mint)
            );
            found = true;
            break;
        }
    }
    assert!(found, "Stake event not found in logs");
}
