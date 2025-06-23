pub mod deposit;
pub mod set;
pub mod staking_pool_initialize;
pub mod withdraw;

use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::pubkey::Pubkey;
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
    Deposit {
        amount: u64,
    },
    Withdraw {
        amount: u64,
    },
    Set {
        new_wind_up_period: Option<u64>,
        new_cool_down_period: Option<u64>,
        new_update_authority: Option<Pubkey>,
    },
}

impl InstructionDiscriminator {
    pub fn to_bytes(&self) -> &[u8; 1] {
        unsafe { &*(self as *const _ as *const [u8; 1]) }
    }
}
