use crate::utils::fixture::{Env, PoolSetup, UserSetup};
use crate::utils::flows::{deposit_yield_into_vault, stake_orca};
use crate::TestContext;

#[test]
fn test_vault_inflation_vulnerability() {
    // Arrange: fresh deployment with empty vault
    let mut staker_ctx = TestContext::new();
    let pool = PoolSetup {
        xorca_supply: 0,
        vault_orca: 0,
        escrowed_orca: 0,
        cool_down_period_s: 3600,
    };
    let attacker_setup = UserSetup {
        staker_orca: 1_000_001,
        staker_xorca: 0,
    };
    let staker_setup = UserSetup {
        staker_orca: 1_000_000,
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

    // // Act: attacker donates 1 ORCA to vault
    deposit_yield_into_vault(&mut attacker_env, 1_000_000, "attacker donates to vault");

    // // Currently, the following fails due to the vault inflation vulnerability.
    // // The InsufficientStakeAmount error is raised.
    // // We will fix this by updating the stake function to handle the vault inflation vulnerability.

    // // Act: staker stakes 1 ORCA to mint xORCA
    stake_orca(&mut staker_env, 1_000_000, "staker attempts to stake");
}
