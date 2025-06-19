pub mod deposit;
pub mod initialize;
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
    Initialize {
        wind_up_period_s: u64,
        cool_down_period_s: u64,
    },
    Deposit {
        amount: u64,
    },
    Withdraw {
        amount: u64,
    },
}

impl InstructionDiscriminator {
    pub fn to_bytes(&self) -> &[u8; 1] {
        unsafe { &*(self as *const _ as *const [u8; 1]) }
    }
}
