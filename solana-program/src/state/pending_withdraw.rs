use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey, sysvars::clock::UnixTimestamp};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct PendingWithdraw {
    discriminator: AccountDiscriminator,
    pub withdrawable_stake_amount: u64,
    pub withdrawable_timestamp: UnixTimestamp,
}

impl PendingWithdraw {
    pub fn seeds<'a>(
        staking_pool: &'a Pubkey,
        unstaker: &'a Pubkey,
        withdraw_index: &'a [u8],
    ) -> Vec<Seed<'a>> {
        vec![
            Seed::from(b"pending_withdraw"),
            Seed::from(staking_pool),
            Seed::from(unstaker),
            Seed::from(withdraw_index),
        ]
    }
}

impl ProgramAccount for PendingWithdraw {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::PendingWithdraw;
}
