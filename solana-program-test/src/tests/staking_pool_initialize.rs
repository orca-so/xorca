use crate::{assert_program_error, assert_program_success};
use crate::{staking_pool_data, TestContext};
use rstest::rstest;
use solana_sdk::pubkey::Pubkey;
use xorca::{
    get_xorca_staking_pool_address, AccountDiscriminator, StakingPool, StakingPoolInitialize,
    XorcaError,
};

#[rstest]
fn test_staking_pool_initialize(
    #[values("Success", "StakingPoolExists", "IncorrectStakingContractAddress")] case: &str,
) {
    let mut ctx = TestContext::new();

    let xorca_staking_pool = if case == "IncorrectAuthorityConfigAddress" {
        Pubkey::new_unique()
    } else {
        get_xorca_staking_pool_address().unwrap().0
    };

    if case == "StakingPoolExists" {
        ctx.write_account(xorca_staking_pool, xorca::ID, staking_pool_data!())
            .unwrap();
    }

    let ix = StakingPoolInitialize {}.instruction();

    let result = ctx.send(ix);

    match case {
        "Success" => {
            assert_program_success!(result);
            let account_after = ctx.get_account::<StakingPool>(xorca_staking_pool).unwrap();
            assert_eq!(
                account_after.data.discriminator,
                AccountDiscriminator::StakingPool
            );
        }
        "ConfigExists" => {
            assert_program_error!(result, XorcaError::IncorrectOwner);
        }
        "IncorrectAuthorityConfigAddress" => {
            assert_program_error!(result, XorcaError::InvalidSeeds);
        }
        _ => panic!("Unknown case: {}", case),
    }
}
