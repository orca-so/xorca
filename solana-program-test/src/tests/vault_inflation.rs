use crate::utils::assert::{assert_mint, assert_token_account, ExpectedMint, ExpectedTokenAccount};
use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::{
    advance_clock_env, deposit_yield_into_vault, do_unstake, do_withdraw, stake_orca,
};
use crate::{TestContext, ORCA_ID, XORCA_ID};
use xorca::TokenAccount;

static ATTACKER_DONATION_AMOUNT: u64 = 1_000_000;

/**
 * Our implementation of virtual assets in the exchange rate calculation (math.rs) disincentivizes vault
 * inflation attacks by guaranteeing a loss for the attacker.
 *
 * This test demonstrates that the implementation is effective at disincentivizing vault inflation attacks.
 */

#[test]
fn test_vault_inflation_vulnerability() {
    // Arrange: fresh deployment with empty vault
    let staker_ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 3600,
    };
    let attacker_setup = UserSetup {
        staker_orca: 1,
        staker_xorca: 0,
    };
    let staker_setup = UserSetup {
        staker_orca: 10_000_000,
        staker_xorca: 0,
    };
    let mut staker_env = Env::new(staker_ctx, &pool, &staker_setup);

    // Create attacker environment using the same SVM instance
    let attacker_ctx = TestContext::new_signer(staker_env.ctx.svm.clone());
    let mut attacker_env = Env::new_user(
        attacker_ctx,
        staker_env.state,
        staker_env.vault,
        &attacker_setup,
    );

    // Act: attacker stakes 0.000001 ORCA to mint xORCA at fresh deploy
    stake_orca(&mut attacker_env, 1, "attacker seed");

    assert_token_account(
        &attacker_env.ctx,
        attacker_env.staker_orca_ata,
        ExpectedTokenAccount {
            owner: &&attacker_env.staker,
            mint: &ORCA_ID,
            amount: 0,
            label: "attacker ORCA after attackerseed",
        },
    );
    assert_token_account(
        &attacker_env.ctx,
        attacker_env.staker_xorca_ata,
        ExpectedTokenAccount {
            owner: &attacker_env.staker,
            mint: &XORCA_ID,
            amount: 1,
            label: "attacker xORCA after attacker seed",
        },
    );
    assert_mint(
        &attacker_env.ctx,
        XORCA_ID,
        ExpectedMint {
            decimals: 6,
            supply: 1,
            mint_authority: &attacker_env.state,
            label: "xORCA mint after attacker seed",
        },
    );
    let attacker_xorca_holdings = 1;
    assert_token_account(
        &attacker_env.ctx,
        attacker_env.vault,
        ExpectedTokenAccount {
            owner: &attacker_env.state,
            mint: &ORCA_ID,
            amount: attacker_xorca_holdings,
            label: "vault after attacker seed",
        },
    );

    // Act: attacker donates 1 ORCA to vault
    deposit_yield_into_vault(&mut attacker_env, 1_000_000, "attacker donates to vault");

    assert_token_account(
        &attacker_env.ctx,
        attacker_env.vault,
        ExpectedTokenAccount {
            owner: &attacker_env.state,
            mint: &ORCA_ID,
            amount: ATTACKER_DONATION_AMOUNT.checked_add(1).unwrap(),
            label: "vault after attacker seed",
        },
    );

    // Act: staker (victim) stakes 1.000001 ORCA that maximizes his loss due to vault inflation.
    // 0.000001 ORCA more and this staker would have minted 2 xORCA instead.
    stake_orca(&mut staker_env, 1_000_001, "staker attempts to stake");
    assert_token_account(
        &staker_env.ctx,
        staker_env.staker_xorca_ata,
        ExpectedTokenAccount {
            owner: &staker_env.staker,
            mint: &XORCA_ID,
            amount: 1,
            label: "victim xORCA after stake",
        },
    );
    assert_token_account(
        &staker_env.ctx,
        staker_env.vault,
        ExpectedTokenAccount {
            owner: &staker_env.state,
            mint: &ORCA_ID,
            amount: ATTACKER_DONATION_AMOUNT
                .checked_add(1)
                .unwrap()
                .checked_add(1_000_001)
                .unwrap(),
            label: "vault after victim stake",
        },
    );
    assert_mint(
        &staker_env.ctx,
        XORCA_ID,
        ExpectedMint {
            decimals: 6,
            supply: 2,
            mint_authority: &staker_env.state,
            label: "xORCA mint after victim stake",
        },
    );

    // Act: attacker unstakes 0.000001 xORCA
    let withdraw_index = 0u8;
    let res_u = do_unstake(
        &mut attacker_env,
        withdraw_index,
        attacker_xorca_holdings,
        0,
    );
    assert!(res_u.is_ok());

    // Act: wait until cooldown passes
    advance_clock_env(&mut attacker_env, pool.cool_down_period_s + 1);

    // Act: attacker withdraws
    let pending_withdraw_account =
        xorca::find_pending_withdraw_pda(&attacker_env.staker, &withdraw_index)
            .unwrap()
            .0;
    let res_w = do_withdraw(&mut attacker_env, pending_withdraw_account, withdraw_index);
    assert!(res_w.is_ok());

    // Assert: attacker should have made a loss
    let attacker_orca_after = attacker_env
        .ctx
        .get_account::<TokenAccount>(attacker_env.staker_orca_ata)
        .unwrap()
        .data
        .amount;
    assert!(
        attacker_orca_after
            < attacker_setup
                .staker_orca
                .checked_add(ATTACKER_DONATION_AMOUNT)
                .unwrap(),
        "attacker should have made a loss",
    );
}
