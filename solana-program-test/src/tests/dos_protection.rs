use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::{TestContext, ORCA_ID, TOKEN_2022_PROGRAM_ID, XORCA_ID};
use xorca::find_pending_withdraw_pda;

// Test that unstake operation succeeds with various amounts of pre-funding
#[test]
fn test_unstake_dos_protection_various_pre_funding() {
    let ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 5_000_000, // Match user's xORCA balance
        vault_orca: 4_000_000,
        escrowed_orca: 0,
        cool_down_period_s: 7 * 24 * 60 * 60,
    };
    let user = UserSetup {
        staker_orca: 0,
        staker_xorca: 5_000_000, // Enough for 5 unstake operations (1M each)
    };
    let mut env = Env::new(ctx, &pool, &user);

    // Test with different amounts of pre-funding
    let pre_funding_amounts = [0, 1, 100, 1000, 10000]; // Various lamport amounts

    for (i, &amount) in pre_funding_amounts.iter().enumerate() {
        let withdraw_index = (i as u8) + 1;
        let (pending_withdraw_pda, _bump) =
            find_pending_withdraw_pda(&env.staker, &withdraw_index).unwrap();

        // Pre-fund the account
        let transfer_ix =
            solana_sdk::system_instruction::transfer(&env.staker, &pending_withdraw_pda, amount);
        env.ctx.sends(&[transfer_ix]).unwrap();

        // Attempt unstake - should succeed (use 1M per iteration)
        let res = {
            let ix = xorca::Unstake {
                unstaker_account: env.staker,
                state_account: env.state,
                vault_account: env.vault,
                pending_withdraw_account: pending_withdraw_pda,
                unstaker_xorca_ata: env.staker_xorca_ata,
                xorca_mint_account: XORCA_ID,
                orca_mint_account: ORCA_ID,
                system_program_account: solana_sdk::system_program::ID,
                spl_token_program_account: crate::TOKEN_PROGRAM_ID,
                token2022_program_account: TOKEN_2022_PROGRAM_ID,
            }
            .instruction(xorca::UnstakeInstructionArgs {
                xorca_unstake_amount: 1_000_000,
                withdraw_index,
            });
            env.ctx.sends(&[ix])
        };

        assert!(
            res.is_ok(),
            "Unstake should succeed with pre-funding amount: {}",
            amount
        );
    }
}
