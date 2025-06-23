use crate::{
    assertions::account::{
        assert_account_address, assert_account_owner, assert_account_role, assert_account_seeds,
        assert_external_account_data, AccountRole,
    },
    cpi::token::TokenMint,
    instructions::INITIAL_UPGRADE_AUTHORITY_ID,
    state::staking_pool::StakingPool,
    util::account::{create_program_account, get_account_info},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::{instructions::InitializeMint2, ID as SPL_TOKEN_PROGRAM_ID};

pub fn process_instruction(
    accounts: &[AccountInfo],
    wind_up_period_s: &u64,
    cool_down_period_s: &u64,
    lst_mint_decimals: &u8,
) -> ProgramResult {
    let payer_account = get_account_info(accounts, 0)?;
    let staking_pool_account = get_account_info(accounts, 1)?;
    let lst_mint_account = get_account_info(accounts, 2)?;
    let stake_token_mint_account = get_account_info(accounts, 3)?;
    let update_authority_account = get_account_info(accounts, 4)?;
    let system_program_account = get_account_info(accounts, 5)?;
    let token_program_account = get_account_info(accounts, 6)?;

    // 1. Payer Account Assertions
    assert_account_role(payer_account, &[AccountRole::Signer, AccountRole::Writable])?;

    // 2. Staking Pool Account Assertions
    assert_account_role(staking_pool_account, &[AccountRole::Writable])?;
    assert_account_owner(staking_pool_account, &SYSTEM_PROGRAM_ID)?;
    let mut staking_pool_seeds = StakingPool::seeds(stake_token_mint_account.key());
    let staking_pool_bump =
        assert_account_seeds(staking_pool_account, &crate::ID, &staking_pool_seeds)?;
    staking_pool_seeds.push(Seed::from(&staking_pool_bump));

    // 3. LST Mint Account Assertions
    let mut lst_token_mint_seeds = vec![
        Seed::from(b"lst_token_mint"),
        Seed::from(stake_token_mint_account.key()),
    ];
    assert_account_role(lst_mint_account, &[AccountRole::Writable])?;
    assert_account_owner(lst_mint_account, &SYSTEM_PROGRAM_ID)?;
    let lst_token_mint_bump = assert_account_seeds(
        lst_mint_account,
        &SPL_TOKEN_PROGRAM_ID,
        &lst_token_mint_seeds,
    )?;
    lst_token_mint_seeds.push(Seed::from(&lst_token_mint_bump));

    // 4. Stake Token Mint Account Assertions
    assert_account_owner(stake_token_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_external_account_data::<TokenMint>(stake_token_mint_account)?;

    // 5. Update Authority Account Assertions
    assert_account_address(update_authority_account, &INITIAL_UPGRADE_AUTHORITY_ID)?;

    // 6. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 7. Token Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Initialize Staking Pool Account Data
    let mut staking_pool_account_data = create_program_account::<StakingPool>(
        system_program_account,
        payer_account,
        staking_pool_account,
        &[staking_pool_seeds.as_slice().into()],
    )?;
    staking_pool_account_data.stake_token_mint = *stake_token_mint_account.key();
    staking_pool_account_data.lst_token_mint = *lst_mint_account.key();
    staking_pool_account_data.wind_up_period_s = *wind_up_period_s;
    staking_pool_account_data.cool_down_period_s = *cool_down_period_s;
    staking_pool_account_data.update_authority = *update_authority_account.key();
    staking_pool_account_data.escrowed_lst_token_amount = 0;

    // Initialize LST Token Mint Account Data
    let initialize_mint_instruction = InitializeMint2 {
        mint: lst_mint_account,
        decimals: *lst_mint_decimals,
        mint_authority: staking_pool_account.key(),
        freeze_authority: None,
    };
    initialize_mint_instruction.invoke_signed(&[staking_pool_seeds.as_slice().into()])?;

    Ok(())
}
