use crate::{
    assertions::account::{
        AccountRole, assert_account_address, assert_account_owner, assert_account_role,
    },
    error::ErrorCode,
    util::account::get_account_info,
};
use pinocchio::{ProgramResult, account_info::AccountInfo, pubkey::Pubkey};
use pinocchio_log::log;
use pinocchio_pubkey::pubkey;
use pinocchio_token::{
    ID as SPL_TOKEN_PROGRAM_ID,
    instructions::{Burn, Transfer},
    state::Mint as SplTokenMint,
    state::TokenAccount as SplTokenAccount,
};

const ORCA_MINT_ID: Pubkey = pubkey!("11111111111111111111111111111111");
const XORCA_MINT_ID: Pubkey = pubkey!("11111111111111111111111111111111");

const EXCHANGE_RATE: u64 = 1; // TODO: update to dynamic exchange rate

pub fn process_instruction(accounts: &[AccountInfo], amount: &u64) -> ProgramResult {
    let withdrawer_account = get_account_info(accounts, 0)?;
    let withdrawer_orca_account = get_account_info(accounts, 1)?;
    let withdrawer_xorca_account = get_account_info(accounts, 2)?;
    let staking_pool_account = get_account_info(accounts, 3)?;
    let staking_pool_orca_account = get_account_info(accounts, 4)?;
    let orca_mint_account = get_account_info(accounts, 5)?;
    let xorca_mint_account = get_account_info(accounts, 6)?;
    let xorca_mint_authority = get_account_info(accounts, 7)?;

    assert_account_role(withdrawer_account, &[AccountRole::Signer])?;
    assert_account_role(withdrawer_orca_account, &[AccountRole::Writable])?;
    assert_account_role(withdrawer_xorca_account, &[AccountRole::Writable])?;
    assert_account_role(staking_pool_orca_account, &[AccountRole::Writable])?;

    assert_account_owner(withdrawer_orca_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(withdrawer_xorca_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(staking_pool_orca_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_address(orca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(xorca_mint_account, &SPL_TOKEN_PROGRAM_ID)?;
    assert_account_owner(xorca_mint_authority, &crate::ID)?;

    let withdrawer_orca_account_data =
        &*SplTokenAccount::from_account_info(&withdrawer_orca_account)?;
    if withdrawer_orca_account_data.owner() != withdrawer_account.key() {
        log!("Error: Withdrawer's orca token account owner does not match withdrawer.");
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if withdrawer_orca_account_data.mint() != &ORCA_MINT_ID {
        log!("Error: Withdrawer orca token account has an unexpected mint.");
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let withdrawer_xorca_account_data =
        &*SplTokenAccount::from_account_info(&withdrawer_xorca_account)?;
    if withdrawer_xorca_account_data.owner() != withdrawer_account.key() {
        log!("Error: Withdrawer's xOrca token account owner does not match withdrawer.");
        return Err(ErrorCode::InvalidAccountData.into());
    }
    if withdrawer_xorca_account_data.mint() != &XORCA_MINT_ID {
        log!("Error: Withdrawer xOrca token account has an unexpected mint.");
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

    let burn_instruction = Burn {
        account: withdrawer_xorca_account,
        mint: xorca_mint_account,
        authority: withdrawer_account, // The owner of the xOrca account
        amount: *amount,
    };
    burn_instruction.invoke()?;
    log!(
        "{} xOrca tokens burned from withdrawer account successfully!",
        *amount
    );

    let orca_to_send = amount
        .checked_div(EXCHANGE_RATE)
        .ok_or(ErrorCode::ArithmeticError)?;

    let transfer_instruction = Transfer {
        from: staking_pool_orca_account,
        to: withdrawer_orca_account,
        authority: withdrawer_account,
        amount: *amount,
    };
    transfer_instruction.invoke()?;
    log!(
        "{} Orca tokens withdrawn into staking pool successfully!",
        orca_to_send
    );

    Ok(())
}
