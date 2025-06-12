use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct XorcaState {
    discriminator: AccountDiscriminator,
    pub escrowed_orca_amount: u64,
    pub xorca_mint: Pubkey,
}

impl XorcaState {
    pub fn seeds<'a>(orca_mint: &'a Pubkey) -> Vec<Seed<'a>> {
        vec![Seed::from(b"xorca_state"), Seed::from(orca_mint)]
    }
}

impl ProgramAccount for XorcaState {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::XorcaState;
}
