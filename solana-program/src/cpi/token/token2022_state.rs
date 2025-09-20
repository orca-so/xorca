use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::pubkey::Pubkey;

/// Token2022 Mint account structure
/// This is identical to the regular TokenMint but used for Token2022 program
#[derive(Debug, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub struct Token2022Mint {
    pub mint_authority_flag: u32,
    pub mint_authority: Pubkey,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority_flag: u32,
    pub freeze_authority: Pubkey,
}

/// Token2022 Account state enum
/// This is identical to the regular TokenAccountState but used for Token2022 program
#[derive(Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum Token2022AccountState {
    /// Account is not yet initialized
    Uninitialized,

    /// Account is initialized; the account owner and/or delegate may perform
    /// permitted operations on this account
    Initialized,

    /// Account has been frozen by the mint freeze authority. Neither the
    /// account owner nor the delegate are able to perform operations on
    /// this account.
    Frozen,
}

/// Token2022 Account structure
/// This is identical to the regular TokenAccount but used for Token2022 program
#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize)]
pub struct Token2022Account {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate_flag: u32,
    pub delegate: Pubkey,
    pub state: Token2022AccountState,
    pub is_native_flag: u32,
    pub native_amount: u64,
    pub delegate_amount: u64,
    pub close_authority_flag: u32,
    pub close_authority: Pubkey,
}
