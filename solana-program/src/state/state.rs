use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount, DEFAULT_ACCOUNT_LEN};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct State {
    pub discriminator: AccountDiscriminator,
    pub escrowed_orca_amount: u64,
    pub cool_down_period_s: u64,
    pub update_authority: Pubkey,
}

impl Default for State {
    fn default() -> Self {
        Self {
            discriminator: AccountDiscriminator::State,
            escrowed_orca_amount: 0,
            update_authority: Pubkey::default(),
            cool_down_period_s: 0,
        }
    }
}

impl State {
    pub fn seeds<'a>() -> Vec<Seed<'a>> {
        vec![Seed::from(b"state")]
    }
}

impl ProgramAccount for State {
    const LEN: usize = DEFAULT_ACCOUNT_LEN;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::State;
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshDeserialize;

    #[test]
    fn test_state_byte_alignment() {
        let expected = State {
            discriminator: AccountDiscriminator::State,
            escrowed_orca_amount: 1000,
            cool_down_period_s: 100,
            update_authority: [1; 32],
        };

        let bytes = borsh::to_vec(&expected).unwrap();
        let actual = State::try_from_slice(&bytes).unwrap();

        assert_eq!(actual.discriminator, expected.discriminator);
        assert_eq!(actual.escrowed_orca_amount, expected.escrowed_orca_amount);
        assert_eq!(actual.update_authority, expected.update_authority);
        assert_eq!(actual.cool_down_period_s, expected.cool_down_period_s);
    }
}
