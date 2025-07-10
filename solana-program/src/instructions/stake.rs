use crate::{
    assertions::account::{
        assert_account_address, assert_account_data_mut, assert_account_owner, assert_account_role,
        assert_account_seeds, assert_external_account_data, make_owner_token_account_assertions,
        AccountRole,
    },
    cpi::token::{TokenMint, ORCA_MINT_ID, XORCA_MINT_ID},
    error::ErrorCode,
    state::state::State,
    util::{account::get_account_info, math::convert_orca_to_xorca},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use pinocchio_token::{
    instructions::{MintTo, Transfer},
    ID as SPL_TOKEN_PROGRAM_ID,
};

pub fn process_instruction(accounts: &[AccountInfo], stake_amount: &u64) -> ProgramResult {
    let staker_account = get_account_info(accounts, 0)?;
    let vault_account = get_account_info(accounts, 1)?;
    let staker_orca_ata = get_account_info(accounts, 2)?;
    let staker_xorca_ata = get_account_info(accounts, 3)?;
    let state_account = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let xorca_mint_account = get_account_info(accounts, 6)?;

    // 1. Staker Account Assertions
    assert_account_role(
        staker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Vault Account Assertions
    let vault_account_seeds = vec![
        Seed::from(state_account.key()),
        Seed::from(SPL_TOKEN_PROGRAM_ID.as_ref()),
        Seed::from(orca_mint_account.key()),
    ];
    assert_account_seeds(
        vault_account,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
        &vault_account_seeds,
    )?;
    let vault_account_data =
        make_owner_token_account_assertions(vault_account, state_account, orca_mint_account)?;

    // 3. Staker Orca ATA Assertions
    let staker_orca_ata_data =
        make_owner_token_account_assertions(staker_orca_ata, staker_account, orca_mint_account)?;
    if staker_orca_ata_data.amount < *stake_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    // 4. Staker xORCA ATA Assertions
    make_owner_token_account_assertions(staker_xorca_ata, staker_account, xorca_mint_account)?;

    // 5. State Account Assertions
    assert_account_owner(state_account, &crate::ID)?;
    let mut state_seeds = State::seeds();
    let state_bump = assert_account_seeds(state_account, &crate::ID, &state_seeds)?;
    state_seeds.push(Seed::from(&state_bump));
    let state = assert_account_data_mut::<State>(state_account)?;

    // 6. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;

    // 7. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(xorca_mint_account, &XORCA_MINT_ID)?;
    let xorca_mint_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;

    // Calculate xOrca to mint
    let non_escrowed_orca_amount = vault_account_data.amount - state.escrowed_orca_amount;
    let xorca_to_mint = convert_orca_to_xorca(
        *stake_amount,
        non_escrowed_orca_amount,
        xorca_mint_data.supply,
    )?;

    // Transfer Orca from staker ATA to vault
    let transfer_instruction = Transfer {
        from: staker_orca_ata,
        to: vault_account,
        authority: staker_account,
        amount: *stake_amount,
    };
    transfer_instruction.invoke()?;

    // Mint xOrca to staker xOrca ATA
    let mint_to_instruction = MintTo {
        mint: xorca_mint_account,
        account: staker_xorca_ata,
        mint_authority: state_account,
        amount: xorca_to_mint,
    };
    mint_to_instruction.invoke_signed(&[state_seeds.as_slice().into()])?;

    Ok(())
}
