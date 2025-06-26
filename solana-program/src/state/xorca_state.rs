use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::instruction::Seed;
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct XorcaState {
    discriminator: AccountDiscriminator,
    value: u64,
}

impl XorcaState {
    pub fn seeds() -> Vec<Seed<'static>> {
        vec![Seed::from(b"xorca_state")]
    }
}

impl ProgramAccount for XorcaState {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::XorcaState;
}
