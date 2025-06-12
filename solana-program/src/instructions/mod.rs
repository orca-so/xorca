pub mod claim;
pub mod deposit;
pub mod staking_pool_initialize;
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
    Deposit { amount: u64 },
    Claim { claim_index: u8 },
    Withdraw { amount: u64 },
}

impl InstructionDiscriminator {
    pub fn to_bytes(&self) -> &[u8; 1] {
        unsafe { &*(self as *const _ as *const [u8; 1]) }
    }
}
