use crate::{
    assertions::account::{
        assert_account_address, assert_account_data, assert_account_owner, assert_account_role,
        assert_external_account_data, make_owner_token_account_assertions, AccountRole,
    },
    cpi::token::{TokenMint, ORCA_MINT_ID, XORCA_MINT_ID},
    error::ErrorCode,
    event::Event,
    state::state::State,
    util::{account::get_account_info, math::convert_orca_to_xorca},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_token::{
    instructions::{MintTo, Transfer},
    ID as SPL_TOKEN_PROGRAM_ID,
};

pub fn process_instruction(accounts: &[AccountInfo], orca_stake_amount: &u64) -> ProgramResult {
    let staker_account = get_account_info(accounts, 0)?;
    let vault_account = get_account_info(accounts, 1)?;
    let staker_orca_ata = get_account_info(accounts, 2)?;
    let staker_xorca_ata = get_account_info(accounts, 3)?;
    let xorca_mint_account = get_account_info(accounts, 4)?;
    let state_account = get_account_info(accounts, 5)?;
    let orca_mint_account = get_account_info(accounts, 6)?;
    let token_program_account = get_account_info(accounts, 7)?;

    // 1. Staker Account Assertions
    assert_account_role(
        staker_account,
        &[AccountRole::Signer, AccountRole::Writable],
    )?;

    // 2. Account Address Assertions
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;
    assert_account_address(xorca_mint_account, &XORCA_MINT_ID)?;
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // 3. Staker Orca ATA Assertions
    let staker_orca_ata_data =
        make_owner_token_account_assertions(staker_orca_ata, staker_account, orca_mint_account)?;
    if staker_orca_ata_data.amount < *orca_stake_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    // 4. Staker xORCA ATA Assertions
    make_owner_token_account_assertions(staker_xorca_ata, staker_account, xorca_mint_account)?;

    // 5. xOrca Mint Account Assertions
    assert_account_role(xorca_mint_account, &[AccountRole::Writable])?;
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    let xorca_mint_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;

    // 7. Orca Mint Account Assertions

    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;

    // 8. Token Program Assertions

    // 6. State Account Assertions
    assert_account_owner(state_account, &crate::ID)?;
    let mut state_seeds = State::seeds();
    let state_view = assert_account_data::<State>(state_account)?;
    State::verify_address_with_bump(state_account, &crate::ID, state_view.bump)
        .map_err(|_| ErrorCode::InvalidSeeds)?;

    // Verify vault address using stored vault_bump
    State::verify_vault_address_with_bump(
        state_account,
        vault_account,
        orca_mint_account,
        state_view.vault_bump,
    )
    .map_err(|_| ErrorCode::InvalidSeeds)?;

    let bump_bytes = [state_view.bump];
    state_seeds.push(Seed::from(&bump_bytes));
    let state = state_view;

    // 2. Vault Account Assertions
    let vault_account_data =
        make_owner_token_account_assertions(vault_account, state_account, orca_mint_account)?;

    // Calculate xOrca to mint
    // Use checked math to guard against vault < escrow (should not happen, but defensive)
    let non_escrowed_orca_amount = vault_account_data
        .amount
        .checked_sub(state.escrowed_orca_amount)
        .ok_or(ErrorCode::InsufficientVaultBacking)?;

    let xorca_to_mint = convert_orca_to_xorca(
        *orca_stake_amount,
        non_escrowed_orca_amount,
        xorca_mint_data.supply,
    )?;

    if xorca_to_mint == 0 {
        return Err(ErrorCode::InsufficientStakeAmount.into());
    }

    // Transfer Orca from staker ATA to vault
    let transfer_instruction = Transfer {
        from: staker_orca_ata,
        to: vault_account,
        authority: staker_account,
        amount: *orca_stake_amount,
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

    Event::Stake {
        orca_stake_amount: orca_stake_amount,
        vault_orca_amount: &vault_account_data.amount,
        vault_escrowed_orca_amount: &state.escrowed_orca_amount,
        xorca_mint_supply: &xorca_mint_data.supply,
        xorca_to_mint: &xorca_to_mint,
    }
    .emit()?;

    Ok(())
}
