use crate::{
    assertions::account::{
        assert_account_address, assert_account_owner, assert_account_role, assert_account_seeds,
        assert_external_account_data, AccountRole,
    },
    cpi::token::TokenMint,
    instructions::INITIAL_UPGRADE_AUTHORITY_ID,
    state::xorca_state::XorcaState,
    util::account::{create_program_account, get_account_info},
};
use pinocchio::{account_info::AccountInfo, instruction::Seed, ProgramResult};
use pinocchio_system::ID as SYSTEM_PROGRAM_ID;
use pinocchio_token::ID as SPL_TOKEN_PROGRAM_ID;

pub fn process_instruction(accounts: &[AccountInfo], cool_down_period_s: &u64) -> ProgramResult {
    let payer_account = get_account_info(accounts, 0)?;
    let xorca_state_account = get_account_info(accounts, 1)?;
    let xorca_mint_account = get_account_info(accounts, 2)?;
    let orca_mint_account = get_account_info(accounts, 3)?;
    let update_authority_account = get_account_info(accounts, 4)?;
    let system_program_account = get_account_info(accounts, 5)?;
    let token_program_account = get_account_info(accounts, 6)?;

    // 1. Payer Account Assertions
    assert_account_role(payer_account, &[AccountRole::Signer, AccountRole::Writable])?;

    // 2. xOrca State Account Assertions
    assert_account_role(xorca_state_account, &[AccountRole::Writable])?;
    assert_account_owner(xorca_state_account, &SYSTEM_PROGRAM_ID)?;
    let mut xorca_state_seeds = XorcaState::seeds(orca_mint_account.key());
    let xorca_state_bump =
        assert_account_seeds(xorca_state_account, &crate::ID, &xorca_state_seeds)?;
    xorca_state_seeds.push(Seed::from(&xorca_state_bump));

    // 3. xOrca Mint Account Assertions
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    let xorca_mint_account_data = assert_external_account_data::<TokenMint>(xorca_mint_account)?;
    assert_account_address(xorca_state_account, &xorca_mint_account_data.mint_authority)?;
    assert_eq!(xorca_mint_account_data.supply, 0);

    // 4. Orca Mint Account Assertions
    assert_account_owner(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_external_account_data::<TokenMint>(orca_mint_account)?;

    // 5. Update Authority Account Assertions
    assert_account_address(update_authority_account, &INITIAL_UPGRADE_AUTHORITY_ID)?;

    // 6. System Program Account Assertions
    assert_account_address(system_program_account, &SYSTEM_PROGRAM_ID)?;

    // 7. Token Account Assertions
    assert_account_address(token_program_account, &SPL_TOKEN_PROGRAM_ID)?;

    // Initialize xOrca State
    let mut xorca_state = create_program_account::<XorcaState>(
        system_program_account,
        payer_account,
        xorca_state_account,
        &[xorca_state_seeds.as_slice().into()],
    )?;
    xorca_state.xorca_mint = *xorca_mint_account.key();
    xorca_state.cool_down_period_s = *cool_down_period_s;
    xorca_state.update_authority = *update_authority_account.key();
    xorca_state.escrowed_orca_amount = 0;

    Ok(())
}
