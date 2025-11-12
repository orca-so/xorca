use crate::{
    fetch_all_maybe_pending_withdraw, fetch_state, find_orca_vault_address,
    find_pending_withdraw_pda, find_state_address, PendingWithdraw, State,
};
use solana_client::rpc_client::RpcClient;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use spl_token_interface::state::{Account, Mint};
use std::str::FromStr;

pub const DEFAULT_MAX_WITHDRAWALS_TO_SEARCH: u8 = 15;
pub const WITHDRAW_INDEX_MAX_UINT: u8 = 255;
pub const ORCA_MINT_ADDRESS: &str = "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE";
pub const XORCA_MINT_ADDRESS: &str = "xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N";
pub const TOKEN_PROGRAM_ADDRESS: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[derive(Debug, Clone)]
pub struct VaultState {
    pub address: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct StakingExchangeRate {
    pub numerator: u64,
    pub denominator: u64,
}

/// Fetches the state account data from the blockchain
///
/// # Arguments
/// * `rpc` - The RPC client to use for fetching data
///
/// # Returns
/// The decoded `State` account data
///
/// # Errors
/// Returns an error if the state account is not found or cannot be decoded
pub fn fetch_state_account_data(rpc: &RpcClient) -> Result<State, std::io::Error> {
    let (state_address, _) = find_state_address()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let decoded = fetch_state(rpc, &state_address)?;
    Ok(decoded.data)
}

/// Fetches the cooldown period (in seconds) from the state account
///
/// # Arguments
/// * `rpc` - The RPC client to use for fetching data
///
/// # Returns
/// The cooldown period in seconds as an `i64`
///
/// # Errors
/// Returns an error if the state account cannot be fetched
pub fn fetch_state_account_cool_down_period_s(rpc: &RpcClient) -> Result<i64, std::io::Error> {
    let state = fetch_state_account_data(rpc)?;
    Ok(state.cool_down_period_s)
}

/// Fetches all pending withdrawals for a given staker
///
/// # Arguments
/// * `rpc` - The RPC client to use for fetching data
/// * `staker` - The public key of the staker
/// * `max_withdrawals_to_search` - Maximum number of withdrawal indices to search (default: 15)
///
/// # Returns
/// A vector of `PendingWithdraw` accounts that exist for the staker
///
/// # Errors
/// Returns an error if:
/// - `max_withdrawals_to_search` is out of range (0-255) or not an integer
/// - Any RPC call fails
pub fn fetch_pending_withdraws_for_staker(
    rpc: &RpcClient,
    staker: &Pubkey,
    max_withdrawals_to_search: Option<u8>,
) -> Result<Vec<PendingWithdraw>, std::io::Error> {
    let max_withdrawals = max_withdrawals_to_search.unwrap_or(DEFAULT_MAX_WITHDRAWALS_TO_SEARCH);

    // Generate all potential pending withdraw addresses
    let mut addresses = Vec::new();
    for i in 0..max_withdrawals {
        let (address, _) = find_pending_withdraw_pda(staker, &i)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        addresses.push(address);
    }

    // Fetch all accounts (including non-existent ones)
    let maybe_accounts = fetch_all_maybe_pending_withdraw(rpc, &addresses)?;

    // Filter out non-existent accounts and extract the data
    let pending_withdraws: Vec<PendingWithdraw> = maybe_accounts
        .into_iter()
        .filter_map(|maybe_account| match maybe_account {
            crate::shared::MaybeAccount::Exists(decoded) => Some(decoded.data),
            crate::shared::MaybeAccount::NotFound(_) => None,
        })
        .collect();

    Ok(pending_withdraws)
}

/// Fetches the vault token account state
///
/// # Arguments
/// * `rpc` - The RPC client to use for fetching data
///
/// # Returns
/// A `VaultState` struct containing the vault's address, owner, mint, and amount
///
/// # Errors
/// Returns an error if the vault account cannot be found or decoded
pub fn fetch_vault_state(rpc: &RpcClient) -> Result<VaultState, std::io::Error> {
    let (state_address, _) = find_state_address()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ADDRESS)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
    let orca_mint = Pubkey::from_str(ORCA_MINT_ADDRESS)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    let (vault_address, _) = find_orca_vault_address(&state_address, &token_program, &orca_mint)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let account = rpc
        .get_account(&vault_address)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Decode the token account data
    let token_account = Account::unpack(&account.data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(VaultState {
        address: vault_address,
        owner: token_account.owner,
        mint: token_account.mint,
        amount: token_account.amount,
    })
}

/// Fetches the total supply of the xORCA mint
///
/// # Arguments
/// * `rpc` - The RPC client to use for fetching data
///
/// # Returns
/// The total supply of xORCA as a `u64`
///
/// # Errors
/// Returns an error if the xORCA mint account cannot be found or decoded
pub fn fetch_xorca_mint_supply(rpc: &RpcClient) -> Result<u64, std::io::Error> {
    let xorca_mint = Pubkey::from_str(XORCA_MINT_ADDRESS)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    let account = rpc
        .get_account(&xorca_mint)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // Decode the mint account data
    let mint = Mint::unpack(&account.data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(mint.supply)
}

/// Calculates the staking exchange rate (ORCA to xORCA)
///
/// The exchange rate is calculated as:
/// - Numerator: vault_amount - escrowed_orca_amount
/// - Denominator: xORCA total supply
///
/// # Arguments
/// * `rpc` - The RPC client to use for fetching data
///
/// # Returns
/// A `StakingExchangeRate` struct containing the numerator and denominator
///
/// # Errors
/// Returns an error if any of the required data cannot be fetched
pub fn fetch_staking_exchange_rate(rpc: &RpcClient) -> Result<StakingExchangeRate, std::io::Error> {
    let state = fetch_state_account_data(rpc)?;
    let vault = fetch_vault_state(rpc)?;
    let numerator = vault
        .amount
        .checked_sub(state.escrowed_orca_amount)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Vault amount is less than escrowed amount",
            )
        })?;
    let denominator = fetch_xorca_mint_supply(rpc)?;

    Ok(StakingExchangeRate {
        numerator,
        denominator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_MAX_WITHDRAWALS_TO_SEARCH, 15);
        assert_eq!(WITHDRAW_INDEX_MAX_UINT, 255);
    }

    #[test]
    fn test_mint_addresses_are_valid() {
        assert!(Pubkey::from_str(ORCA_MINT_ADDRESS).is_ok());
        assert!(Pubkey::from_str(XORCA_MINT_ADDRESS).is_ok());
        assert!(Pubkey::from_str(TOKEN_PROGRAM_ADDRESS).is_ok());
    }
}
