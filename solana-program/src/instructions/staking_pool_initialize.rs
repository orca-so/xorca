use crate::{
    assertions::account::{
        assert_account_address, assert_account_role, assert_account_seeds, AccountRole,
    },
    state::staking_pool::StakingPool,
    util::account::{create_program_account, get_account_info},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_log::log;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;

pub fn process_instruction(accounts: &[AccountInfo]) -> ProgramResult {
    let payer_account = get_account_info(accounts, 0)?;
    let staking_pool_account = get_account_info(accounts, 1)?;
    let system_program_account = get_account_info(accounts, 2)?;

    assert_account_role(payer_account, &[AccountRole::Signer, AccountRole::Writable])?;
    assert_account_role(staking_pool_account, &[AccountRole::Writable])?;
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    let mut staking_pool_seeds = StakingPool::seeds();
    let staking_pool_bump =
        assert_account_seeds(staking_pool_account, &crate::ID, &staking_pool_seeds)?;
    staking_pool_seeds.push(Seed::from(&staking_pool_bump));

    create_program_account::<StakingPool>(
        system_program_account,
        payer_account,
        staking_pool_account,
        &[staking_pool_seeds.as_slice().into()],
    )?;

    log!("Staking pool initialized successfully!");
    Ok(())
}
