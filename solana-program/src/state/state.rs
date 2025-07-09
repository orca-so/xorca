use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct State {
    discriminator: AccountDiscriminator,
    pub escrowed_orca_amount: u64,
    pub update_authority: Pubkey,
    pub cool_down_period_s: u64,
}

impl State {
    pub fn seeds<'a>() -> Vec<Seed<'a>> {
        vec![Seed::from(b"state")]
    }
}

impl ProgramAccount for State {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::State;
}
