use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct PendingWithdraw {
    discriminator: AccountDiscriminator,
    pub withdrawable_orca_amount: u64,
    pub withdrawable_timestamp: i64,
}

impl PendingWithdraw {
    pub fn seeds<'a>(
        xorca_state: &'a Pubkey,
        unstaker: &'a Pubkey,
        withdraw_index: &'a [u8],
    ) -> Vec<Seed<'a>> {
        vec![
            Seed::from(b"pending_withdraw"),
            Seed::from(xorca_state),
            Seed::from(unstaker),
            Seed::from(withdraw_index),
        ]
    }
}

impl ProgramAccount for PendingWithdraw {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::PendingWithdraw;
}
