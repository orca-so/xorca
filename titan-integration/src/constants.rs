use solana_pubkey::Pubkey;
use titan_integration_template::trading_venue::token_info::TokenInfo;

// PROGRAM IDS
pub const XORCA_STAKING_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT");

pub const TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// TOKEN INFO
pub const ORCA_TOKEN_INFO: TokenInfo = TokenInfo {
    pubkey: Pubkey::from_str_const("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE"),
    decimals: 6,
    is_token_2022: false,
    transfer_fee: None,
    maximum_fee: None,
};

pub const XORCA_TOKEN_INFO: TokenInfo = TokenInfo {
    pubkey: Pubkey::from_str_const("xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N"),
    decimals: 6,
    is_token_2022: false,
    transfer_fee: None,
    maximum_fee: None,
};

// PUBLIC KEYS
pub const STATE_KEY: Pubkey =
    Pubkey::from_str_const("CSqKhyW1cpdyjheAx5HXx4ibcnYrzpL5JywEMAkZixBK");

pub const VAULT_KEY: Pubkey =
    Pubkey::from_str_const("Ce5j11WAsSzM3nkzrw4Kw6v6ic3nbyqpv5eywjYKeKc5");
