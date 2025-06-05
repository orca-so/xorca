use crate::{
    assertions::account::{
        AccountRole, assert_account_address, assert_account_owner, assert_account_role,
    },
    error::ErrorCode,
    util::account::get_account_info,
};
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::{Pubkey, find_program_address},
    seeds,
};
use pinocchio_log::log;
use pinocchio_pubkey::pubkey;
use pinocchio_token::{
    ID as SPL_TOKEN_PROGRAM_ID,
    instructions::{MintTo, Transfer},
    state::Mint as SplTokenMint,
    state::TokenAccount as SplTokenAccount,
};

const ORCA_MINT_ID: Pubkey = pubkey!("11111111111111111111111111111111");
const XORCA_MINT_ID: Pubkey = pubkey!("11111111111111111111111111111111");

const EXCHANGE_RATE: u64 = 1; // TODO: update to dynamic exchange rate

pub fn process_instruction(accounts: &[AccountInfo], amount: &u64) -> ProgramResult {
    let depositor_account = get_account_info(accounts, 0)?;
    let depositor_orca_account = get_account_info(accounts, 1)?;
    let depositor_xorca_account = get_account_info(accounts, 2)?;
    let staking_pool_account = get_account_info(accounts, 3)?;
    let staking_pool_orca_account = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let xorca_mint_account = get_account_info(accounts, 6)?;
    let xorca_mint_authority = get_account_info(accounts, 7)?;

    assert_account_role(depositor_account, &[AccountRole::Signer])?;
    assert_account_role(depositor_orca_account, &[AccountRole::Writable])?;
    assert_account_role(depositor_xorca_account, &[AccountRole::Writable])?;
    assert_account_role(staking_pool_orca_account, &[AccountRole::Writable])?;

    assert_account_owner(depositor_orca_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(depositor_xorca_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(staking_pool_orca_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(xorca_mint_authority, &crate::ID)?;

    let depositor_orca_account_data =
        &*SplTokenAccount::from_account_info(&depositor_orca_account)?;
    if depositor_orca_account_data.owner() != depositor_account.key() {
        log!("Error: Depositor's orca token account owner does not match depositor.");
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if depositor_orca_account_data.mint() != &ORCA_MINT_ID {
        log!("Error: Depositor orca token account has an unexpected mint.");
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let depositor_xorca_account_data =
        &*SplTokenAccount::from_account_info(&depositor_xorca_account)?;
    if depositor_xorca_account_data.owner() != depositor_account.key() {
        log!("Error: Depositor's xOrca token account owner does not match depositor.");
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if depositor_xorca_account_data.mint() != &XORCA_MINT_ID {
        log!("Error: Depositor xOrca token account has an unexpected mint.");
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let staking_pool_orca_account_data =
        &*SplTokenAccount::from_account_info(&staking_pool_orca_account)?;
    if staking_pool_orca_account_data.owner() != staking_pool_account.key() {
        log!("Error: Staking pool's orca token account owner does not match staking pool.");
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if staking_pool_orca_account_data.mint() != &ORCA_MINT_ID {
        log!("Error: Staking pool orca token account has an unexpected mint.");
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let xorca_mint_account_data = SplTokenMint::from_account_info(&xorca_mint_account)?;
    if let Some(mint_authority_pubkey_from_data) = xorca_mint_account_data.mint_authority() {
        if mint_authority_pubkey_from_data != xorca_mint_authority.key() {
            log!("Error: xORCA mint authority from data does not match provided PDA.");
            return Err(ErrorCode::InvalidAccountData.into());
        }
    } else {
        log!("Error: xORCA mint has no mint authority, but it should have one.");
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if xorca_mint_account.key() != &XORCA_MINT_ID {
        log!("Error: xORCA mint account has an unexpected ID.");
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let transfer_instruction = Transfer {
        from: depositor_orca_account,
        to: staking_pool_orca_account,
        authority: depositor_account,
        amount: *amount,
    };
    transfer_instruction.invoke()?;
    log!(
        "{} Orca tokens deposited into staking pool successfully!",
        *amount
    );

    let xorca_to_mint = amount
        .checked_mul(EXCHANGE_RATE)
        .ok_or(ErrorCode::ArithmeticError)?;

    let mint_to_instruction = MintTo {
        mint: xorca_mint_account,
        account: depositor_xorca_account,
        mint_authority: xorca_mint_authority,
        amount: xorca_to_mint,
    };

    let xorca_mint_authority_seeds = vec![Seed::from(b"xorca_mint_authority")];
    let seed_bytes = xorca_mint_authority_seeds
        .iter()
        .map(|seed| seed.as_ref())
        .collect::<Vec<&[u8]>>();
    let (address, bump) = find_program_address(&seed_bytes, &crate::ID);
    if &address != xorca_mint_authority.key() {
        log!("Error: xORCA mint authority address does not match expected address.");
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let bump_slice = [bump];
    let seed_array = seeds!(
        b"xorca_mint_authority",
        staking_pool_account.key().as_ref(),
        &bump_slice
    );
    let pda_signer_instance = Signer::from(&seed_array);
    mint_to_instruction.invoke_signed(&[pda_signer_instance])?;

    log!(
        "{} xOrca tokens minted to user successfully!",
        xorca_to_mint
    );

    Ok(())
}
