pub mod stake;
pub mod staking_pool_initialize;
pub mod unstake;
pub mod withdraw;

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use strum::{Display, EnumDiscriminants, FromRepr};

#[derive(
    Debug, Clone, BorshSerialize, BorshDeserialize, ShankInstruction, Display, EnumDiscriminants,
)]
#[strum_discriminants(
    name(InstructionDiscriminator),
    derive(BorshSerialize, BorshDeserialize, FromRepr)
)]
pub enum Instruction {
    StakingPoolInitialize,
    #[account(0, writable, signer, name = "staker_account")]
    #[account(1, writable, name = "staking_pool_account")]
    #[account(2, writable, name = "staking_pool_stake_token_account")]
    #[account(3, writable, name = "pending_claim_account")]
    #[account(4, writable, name = "staker_stake_token_account")]
    #[account(5, name = "stake_token_mint_account")]
    #[account(6, name = "system_program_account")]
    #[account(7, name = "token_program_account")]
    Stake {
        stake_amount: u64,
        claim_index: u8,
    },
    #[account(0, writable, signer, name = "unstaker_account")]
    #[account(1, writable, name = "staking_pool_account")]
    #[account(2, writable, name = "staking_pool_stake_token_account")]
    #[account(3, writable, name = "pending_withdraw_account")]
    #[account(4, writable, name = "unstaker_lst_account")]
    #[account(5, name = "lst_mint_account")]
    #[account(6, name = "stake_token_mint_account")]
    #[account(7, name = "system_program_account")]
    #[account(8, name = "token_program_account")]
    Unstake {
        unstake_amount: u64,
        withdraw_index: u8,
    },
    #[account(0, writable, signer, name = "unstaker_account")]
    #[account(1, writable, name = "staking_pool_account")]
    #[account(2, writable, name = "pending_withdraw_account")]
    #[account(3, writable, name = "unstaker_stake_token_account")]
    #[account(4, writable, name = "staking_pool_stake_token_account")]
    #[account(5, name = "stake_token_mint_account")]
    #[account(6, name = "system_program_account")]
    #[account(7, name = "token_program_account")]
    Withdraw {
        withdraw_index: u8,
    },
}

impl InstructionDiscriminator {
    pub fn to_bytes(&self) -> &[u8; 1] {
        unsafe { &*(self as *const _ as *const [u8; 1]) }
    }
}
