pub mod initialize;
pub mod set;
pub mod stake;
pub mod unstake;
pub mod withdraw;
use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::pubkey;

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use strum::{Display, EnumDiscriminants, FromRepr};

pub const INITIAL_UPGRADE_AUTHORITY_ID: Pubkey = pubkey!("11111111111111111111111111111111"); // TODO: replace with actual initial upgrade authority

#[derive(
    Debug, Clone, BorshSerialize, BorshDeserialize, ShankInstruction, Display, EnumDiscriminants,
)]
#[strum_discriminants(
    name(InstructionDiscriminator),
    derive(BorshSerialize, BorshDeserialize, FromRepr)
)]

pub enum Instruction {
    #[account(0, writable, signer, name = "staker_account")]
    #[account(1, writable, name = "vault_account")]
    #[account(2, writable, name = "staker_orca_ata")]
    #[account(3, writable, name = "staker_xorca_ata")]
    #[account(4, writable, name = "xorca_mint_account")]
    #[account(5, name = "state_account")]
    #[account(6, name = "orca_mint_account")]
    #[account(7, name = "token_program_account")]
    Stake { stake_amount: u64 },

    #[account(0, writable, signer, name = "unstaker_account")]
    #[account(1, writable, name = "state_account")]
    #[account(2, writable, name = "vault_account")]
    #[account(3, writable, name = "pending_withdraw_account")]
    #[account(4, writable, name = "unstaker_lst_account")]
    #[account(5, name = "xorca_mint_account")]
    #[account(6, name = "orca_mint_account")]
    #[account(7, name = "system_program_account")]
    #[account(8, name = "token_program_account")]
    Unstake {
        unstake_amount: u64,
        withdraw_index: u8,
    },

    #[account(0, writable, signer, name = "unstaker_account")]
    #[account(1, writable, name = "state_account")]
    #[account(2, writable, name = "pending_withdraw_account")]
    #[account(3, writable, name = "unstaker_orca_ata")]
    #[account(4, writable, name = "vault_account")]
    #[account(5, name = "orca_mint_account")]
    #[account(6, name = "system_program_account")]
    #[account(7, name = "token_program_account")]
    Withdraw { withdraw_index: u8 },

    #[account(0, writable, signer, name = "payer_account")]
    #[account(1, writable, name = "state_account")]
    #[account(2, name = "xorca_mint_account")]
    #[account(3, name = "orca_mint_account")]
    #[account(4, name = "update_authority_account")]
    #[account(5, name = "system_program_account")]
    Initialize { cool_down_period_s: u64 },

    #[account(0, writable, signer, name = "update_authority_account")]
    #[account(1, writable, name = "state_account")]
    Set {
        new_cool_down_period: Option<u64>,
        new_update_authority: Option<Pubkey>,
    },
}

impl InstructionDiscriminator {
    pub fn to_bytes(&self) -> &[u8; 1] {
        unsafe { &*(self as *const _ as *const [u8; 1]) }
    }
}
