use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, assert_external_account_data, make_owner_token_account_assertions,
        AccountRole,
    },
    cpi::{
        system::get_current_unix_timestamp,
        token::{TokenAccount, TokenMint},
    },
    error::ErrorCode,
    state::{pending_claim::PendingClaim, staking_pool::StakingPool},
    util::{account::get_account_info, math::convert_stake_token_to_lst},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::{instructions::MintTo, ID as SPL_TOKEN_PROGRAM_ID};

pub fn process_instruction(accounts: &[AccountInfo], claim_index: &u8) -> ProgramResult {
    let staker_account = get_account_info(accounts, 0)?;
    let staking_pool_account = get_account_info(accounts, 1)?;
    let pending_claim_account = get_account_info(accounts, 2)?;
    let staker_lst_account = get_account_info(accounts, 3)?;
    let lst_mint_account = get_account_info(accounts, 4)?;
    let stake_token_mint_account = get_account_info(accounts, 5)?;
    let staking_pool_stake_token_account = get_account_info(accounts, 6)?;
    let system_program_account = get_account_info(accounts, 7)?;
    let token_program_account = get_account_info(accounts, 8)?;

    // 1. Staker Account Assertions
    assert_account_role(
        staker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Staking Pool Account Assertions
    assert_account_role(staking_pool_account, &[AccountRole::Writable])?;
    assert_account_owner(staking_pool_account, &crate::ID)?;
    let mut staking_pool_seeds = StakingPool::seeds(stake_token_mint_account.key());
    let staking_pool_bump =
        assert_account_seeds(staking_pool_account, &crate::ID, &staking_pool_seeds)?;
    staking_pool_seeds.push(Seed::from(&staking_pool_bump));
    let mut staking_pool_data = assert_account_data_mut::<StakingPool>(staking_pool_account)?;

    // 3. Pending Claim Account Assertions
    assert_account_role(pending_claim_account, &[AccountRole::Writable])?;
    assert_account_owner(pending_claim_account, &crate::ID)?;
    let claim_index_bytes = [*claim_index];
    let pending_claim_seeds = PendingClaim::seeds(
        staking_pool_account.key(),
        staker_account.key(),
        &claim_index_bytes,
    );
    assert_account_seeds(pending_claim_account, &crate::ID, &pending_claim_seeds)?;
    let pending_claim_data = assert_account_data_mut::<PendingClaim>(pending_claim_account)?;

    // 4. Staker LST Account Assertions
    make_owner_token_account_assertions(staker_lst_account, staker_account, lst_mint_account)?;

    // 5. LST Mint Account Assertions
    assert_account_owner(staker_lst_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(staker_lst_account, &staking_pool_data.lst_token_mint)?;
    let lst_mint_data = assert_external_account_data::<TokenMint>(lst_mint_account)?;

    // 6. Staking Pool Stake Token Account Assertions
    let staking_pool_stake_token_seeds = vec![
        Seed::from(staking_pool_account.key()),
        Seed::from(SPL_TOKEN_PROGRAM_ID.as_ref()),
        Seed::from(staking_pool_data.stake_token_mint.as_ref()),
    ];
    assert_account_seeds(
        staking_pool_stake_token_account,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
        &staking_pool_stake_token_seeds,
    )?;
    let staking_pool_stake_token_data =
        assert_external_account_data::<TokenAccount>(staking_pool_stake_token_account)?;

    // 7. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 8. Token Program Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Validate pending claim
    let current_unix_timestamp = get_current_unix_timestamp()?;
    if current_unix_timestamp < pending_claim_data.claimable_timestamp {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // Calculate LST to mint
    let non_escrowed_stake_token_amount =
        staking_pool_stake_token_data.amount - staking_pool_data.escrowed_stake_token_amount;
    let lst_to_mint = convert_stake_token_to_lst(
        pending_claim_data.stake_amount,
        non_escrowed_stake_token_amount,
        lst_mint_data.supply,
    )?;

    // Mint LST to staker
    let mint_to_instruction = MintTo {
        mint: lst_mint_account,
        account: staker_lst_account,
        mint_authority: staking_pool_account,
        amount: lst_to_mint,
    };
    mint_to_instruction.invoke_signed(&[staking_pool_seeds.as_slice().into()])?;

    // Remove tokens from escrow
    staking_pool_data.escrowed_stake_token_amount -= pending_claim_data.stake_amount;

    // TODO: Close Pending Claim account and refund rent to staker
    Ok(())
}
