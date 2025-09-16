use crate::{
    assertions::account::{
        assert_account_address, assert_account_owner, assert_account_role, assert_account_seeds,
        assert_external_account_data, AccountRole,
    },
    cpi::token::{TokenMint, ORCA_MINT_ID, XORCA_MINT_ID},
    error::ErrorCode,
    state::state::State,
    util::account::{create_program_account_borsh, get_account_info},
    DEPLOYER_ADDRESS,
};
use pinocchio::{
    account_info::AccountInfo, instruction::Seed, pubkey::find_program_address, ProgramResult,
};
use pinocchio_associated_token_account::{
    instructions::CreateIdempotent as CreateAtaIdempotent, ID as ASSOCIATED_TOKEN_PROGRAM_ID,
};
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::ID as SPL_TOKEN_PROGRAM_ID;

pub fn process_instruction(accounts: &[AccountInfo], cool_down_period_s: &i64) -> ProgramResult {
    let payer_account = get_account_info(accounts, 0)?;
    let state_account = get_account_info(accounts, 1)?;
    let xorca_mint_account = get_account_info(accounts, 2)?;
    let orca_mint_account = get_account_info(accounts, 3)?;
    let update_authority_account = get_account_info(accounts, 4)?;
    let system_program_account = get_account_info(accounts, 5)?;
    let vault_account = get_account_info(accounts, 6)?;
    let token_program_account = get_account_info(accounts, 7)?;
    let associated_token_program_account = get_account_info(accounts, 8)?;

    // 1. Payer Account Assertions
    assert_account_role(payer_account, &[AccountRole::Signer, AccountRole::Writable])?;
    
    // 1.1. Deployer Authorization - only the deployer can call initialize
    if payer_account.key() != &DEPLOYER_ADDRESS {
        return Err(ErrorCode::UnauthorizedDeployerAccess.into());
    }

    // 2. xOrca State Account Assertions
    assert_account_role(state_account, &[AccountRole::Writable])?;
    let mut state_seeds = State::seeds();
    let state_bump = assert_account_seeds(state_account, &crate::ID, &state_seeds)?;
    state_seeds.push(Seed::from(&state_bump));

    if state_account.data_len() > 0 {
        return Err(ErrorCode::StateAccountAlreadyInitialized.into());
    }
    assert_account_owner(state_account, &SYSTEM_PROGRAM_ID)?;

    // 3. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    let xorca_mint_account_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;
    assert_account_address(state_account, &xorca_mint_account_data.mint_authority)?;
    if xorca_mint_account_data.supply != 0 {
        return Err(ErrorCode::InvalidAccountData.into());
    }
    assert_account_address(xorca_mint_account, &XORCA_MINT_ID)?;

    // Verify mint authority is this program
    if xorca_mint_account_data.mint_authority_flag != 1 {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // Verify freeze authority is NULL (no freeze authority)
    if xorca_mint_account_data.freeze_authority_flag != 0 {
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if xorca_mint_account_data.freeze_authority != pinocchio::pubkey::Pubkey::default() {
        return Err(ErrorCode::InvalidAccountData.into());
    }
    // Enforce both ORCA and xORCA mints have 6 decimals
    if xorca_mint_account_data.decimals != 6 {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // 4. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    let orca_mint_account_data = assert_external_account_data::<TokenMint>(orca_mint_account)?;
    assert_account_address(orca_mint_account, &ORCA_MINT_ID)?;
    if orca_mint_account_data.decimals != 6 {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // 5. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 6. Vault Account Assertions
    assert_account_role(vault_account, &[AccountRole::Writable])?;
    assert_account_owner(vault_account, &SYSTEM_PROGRAM_ID)?;

    // 7. Token Program Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // 8. Associated Token Program Account Assertions
    assert_account_address(
        associated_token_program_account,
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )?;

    // Calculate vault bump for future verification
    // let vault_seeds = crate::pda::vault_seeds(state_account.key());
    let (_, vault_bump) = find_program_address(
        &crate::pda::seeds::vault_seeds_raw(
            state_account.key(),
            &SPL_TOKEN_PROGRAM_ID,
            orca_mint_account.key(),
        ),
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    );

    // Verify vault address using centralized seeds
    let vault_seeds: Vec<Seed> = crate::pda::seeds::vault_seeds(
        state_account.key(),
        &SPL_TOKEN_PROGRAM_ID,
        orca_mint_account.key(),
    );
    assert_account_seeds(vault_account, &ASSOCIATED_TOKEN_PROGRAM_ID, &vault_seeds)?;

    // Create the State struct
    let mut state_data = State::default();
    if *cool_down_period_s < 0 {
        return Err(ErrorCode::InvalidCoolDownPeriod.into());
    }
    state_data.cool_down_period_s = *cool_down_period_s;
    state_data.bump = state_bump[0];
    state_data.vault_bump = vault_bump;
    state_data.update_authority = *update_authority_account.key();

    create_program_account_borsh(
        payer_account,
        state_account,
        &[state_seeds.as_slice().into()],
        &state_data,
    )?;

    // Create the vault ATA using pinocchio ATA
    CreateAtaIdempotent {
        funding_account: payer_account,
        account: vault_account,
        wallet: state_account,
        mint: orca_mint_account,
        system_program: system_program_account,
        token_program: token_program_account,
    }
    .invoke()?;

    Ok(())
}
