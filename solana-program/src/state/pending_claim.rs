use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct PendingClaim {
    discriminator: AccountDiscriminator,
    pub stake_amount: u64,
    pub claimable_timestamp: i64,
}

impl PendingClaim {
    pub fn seeds<'a>(
        staking_pool: &'a Pubkey,
        staker: &'a Pubkey,
        claim_index: &'a [u8],
    ) -> Vec<Seed<'a>> {
        vec![
            Seed::from(b"pending_claim"),
            Seed::from(staking_pool),
            Seed::from(staker),
            Seed::from(claim_index),
        ]
    }
}

impl ProgramAccount for PendingClaim {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::PendingClaim;
}
