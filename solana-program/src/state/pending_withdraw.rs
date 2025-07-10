use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount, DEFAULT_ACCOUNT_LEN};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct PendingWithdraw {
    discriminator: AccountDiscriminator,
    pub withdrawable_orca_amount: u64,
    pub withdrawable_timestamp: i64,
}

impl PendingWithdraw {
    pub fn seeds<'a>(
        state: &'a Pubkey,
        unstaker: &'a Pubkey,
        withdraw_index: &'a [u8],
    ) -> Vec<Seed<'a>> {
        vec![
            Seed::from(b"pending_withdraw"),
            Seed::from(state),
            Seed::from(unstaker),
            Seed::from(withdraw_index),
        ]
    }
}

impl ProgramAccount for PendingWithdraw {
    const LEN: usize = DEFAULT_ACCOUNT_LEN;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::PendingWithdraw;
}
