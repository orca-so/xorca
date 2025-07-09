use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, assert_external_account_data, make_owner_token_account_assertions,
        AccountRole,
    },
    cpi::token::TokenMint,
    error::ErrorCode,
    state::xorca_state::XorcaState,
    util::{account::get_account_info, math::convert_stake_token_to_lst},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::{
    instructions::{MintTo, Transfer},
    ID as SPL_TOKEN_PROGRAM_ID,
};

pub fn process_instruction(accounts: &[AccountInfo], stake_amount: &u64) -> ProgramResult {
    let staker_account = get_account_info(accounts, 0)?;
    let xorca_state_account = get_account_info(accounts, 1)?;
    let vault_account = get_account_info(accounts, 2)?;
    let staker_orca_ata = get_account_info(accounts, 3)?;
    let staker_xorca_ata = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let xorca_mint_account = get_account_info(accounts, 6)?;
    let system_program_account = get_account_info(accounts, 7)?;
    let token_program_account = get_account_info(accounts, 8)?;

    // 1. Staker Account Assertions
    assert_account_role(
        staker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. xOrca State Account Assertions
    assert_account_role(xorca_state_account, &[AccountRole::Writable])?;
    assert_account_owner(xorca_state_account, &crate::ID)?;
    let mut xorca_state_seeds = XorcaState::seeds(orca_mint_account.key());
    let xorca_state_bump =
        assert_account_seeds(xorca_state_account, &crate::ID, &xorca_state_seeds)?;
    xorca_state_seeds.push(Seed::from(&xorca_state_bump));
    let xorca_state = assert_account_data_mut::<XorcaState>(xorca_state_account)?;

    // 3. Vault Account Assertions
    let vault_account_seeds = vec![
        Seed::from(xorca_state_account.key()),
        Seed::from(SPL_TOKEN_PROGRAM_ID.as_ref()),
        Seed::from(orca_mint_account.key()),
    ];
    assert_account_seeds(
        vault_account,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
        &vault_account_seeds,
    )?;
    let vault_account_data =
        make_owner_token_account_assertions(vault_account, xorca_state_account, orca_mint_account)?;

    // 4. Staker Orca ATA Assertions
    let staker_orca_ata_data =
        make_owner_token_account_assertions(staker_orca_ata, staker_account, orca_mint_account)?;
    if staker_orca_ata_data.amount < *stake_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    // 5. Staker xORCA ATA Assertions
    make_owner_token_account_assertions(staker_xorca_ata, staker_account, xorca_mint_account)?;

    // 6. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;

    // 7. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(xorca_mint_account, &xorca_state.xorca_mint)?;
    let xorca_mint_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;

    // 8. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 9. Token Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Calculate LST to mint
    let non_escrowed_orca_amount = vault_account_data.amount - xorca_state.escrowed_orca_amount;
    let xorca_to_mint = convert_stake_token_to_lst(
        *stake_amount,
        non_escrowed_orca_amount,
        xorca_mint_data.supply,
    )?;

    // Transfer stake tokens from staker ATA to xOrca state ATA
    let transfer_instruction = Transfer {
        from: staker_orca_ata,
        to: vault_account,
        authority: staker_account,
        amount: *stake_amount,
    };
    transfer_instruction.invoke()?;

    // Mint LST to staker LST ATA
    let mint_to_instruction = MintTo {
        mint: xorca_mint_account,
        account: staker_xorca_ata,
        mint_authority: xorca_state_account,
        amount: xorca_to_mint,
    };
    mint_to_instruction.invoke_signed(&[xorca_state_seeds.as_slice().into()])?;

    Ok(())
}
