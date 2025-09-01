use super::{AccountDiscriminator, ProgramAccount};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use pinocchio_pubkey::derive_address;
use shank::ShankAccount;

use crate::error::ErrorCode;

const STATE_ACCOUNT_LEN: usize = 2048;

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct State {
    pub discriminator: AccountDiscriminator, // 1 byte
    // Explicit padding to ensure that the next field (u64) is 8-byte aligned
    // in memory when #[repr(C)] is used.
    // Calculation: use 5 bytes + 1-byte bump + 1-byte vault_bump to reach 8-byte alignment.
    pub padding1: [u8; 5],
    // Cached bump for PDA derivation of the state account.
    pub bump: u8, // 1 byte
    // Cached bump for vault ATA derivation
    pub vault_bump: u8,            // 1 byte
    pub escrowed_orca_amount: u64, // 8 bytes
    pub cool_down_period_s: i64,   // 8 bytes
    pub update_authority: Pubkey,  // 32 bytes
    // STATE_ACCOUNT_LEN (2048 bytes) - (1 + 5 + 1 + 1 + 8 + 8 + 32) = 1992 bytes.
    pub padding2: [u8; 1992],
}

impl Default for State {
    fn default() -> Self {
        Self {
            discriminator: AccountDiscriminator::State,
            padding1: [0; 5],
            bump: 0,
            vault_bump: 0,
            escrowed_orca_amount: 0,
            update_authority: Pubkey::default(),
            cool_down_period_s: 0,
            padding2: [0; 1992],
        }
    }
}

impl State {
    /// Verify the pending withdraw PDA address using pinocchio-pubkey's derive_address with stored bump
    pub fn verify_address_with_bump(
        account: &pinocchio::account_info::AccountInfo,
        program_id: &Pubkey,
        stored_bump: u8,
    ) -> Result<(), ErrorCode> {
        let derived_address = derive_address(&[b"state"], Some(stored_bump), program_id);
        if account.key() != &derived_address {
            return Err(ErrorCode::InvalidSeeds.into());
        }
        Ok(())
    }

    /// Get seeds for backward compatibility with existing assert_account_seeds calls
    pub fn seeds<'a>() -> Vec<Seed<'a>> {
        vec![Seed::from(b"state")]
    }

    /// Get vault seeds for ATA derivation
    pub fn vault_seeds<'a>(
        state_account: &'a pinocchio::account_info::AccountInfo,
        orca_mint: &'a pinocchio::account_info::AccountInfo,
    ) -> Vec<Seed<'a>> {
        vec![
            Seed::from(state_account.key()),
            Seed::from(pinocchio_token::ID.as_ref()),
            Seed::from(orca_mint.key()),
        ]
    }

    /// Verify the vault ATA address using pinocchio-pubkey's derive_address with stored bump
    pub fn verify_vault_address_with_bump(
        &self,
        state_account: &pinocchio::account_info::AccountInfo,
        vault_account: &pinocchio::account_info::AccountInfo,
        orca_mint: &pinocchio::account_info::AccountInfo,
        stored_vault_bump: u8,
    ) -> Result<(), ErrorCode> {
        let vault_seeds = [
            state_account.key().as_ref(),
            pinocchio_token::ID.as_ref(),
            orca_mint.key().as_ref(),
        ];
        let derived_address = derive_address(
            &vault_seeds,
            Some(stored_vault_bump),
            &pinocchio_associated_token_account::ID,
        );
        if vault_account.key() != &derived_address {
            return Err(ErrorCode::InvalidSeeds.into());
        }
        Ok(())
    }
}

impl ProgramAccount for State {
    const LEN: usize = STATE_ACCOUNT_LEN;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::State;
}

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshDeserialize;
    use std::mem::size_of;

    #[test]
    fn test_state_byte_alignment() {
        // Use distinct non-zero values for fields and padding to ensure all bytes
        // are correctly serialized/deserialized and reinterpreted.
        let expected = State {
            discriminator: AccountDiscriminator::State,
            padding1: [0xAA; 5],
            bump: 0x42,
            vault_bump: 0x43,
            escrowed_orca_amount: 0x1122334455667788,
            cool_down_period_s: 7 * 24 * 60 * 60,
            update_authority: Pubkey::default(),
            padding2: [0xCC; 1992],
        };

        // 1. Serialize the struct using Borsh.
        // Because we added explicit padding fields that match #[repr(C)]'s
        // expected padding, the Borsh output will now have these padding bytes.
        let bytes = borsh::to_vec(&expected).unwrap();

        // ASSERTION 1: Check if the Borsh-serialized length matches the expected LEN.
        assert_eq!(
            bytes.len(),
            State::LEN,
            "Borsh serialized length mismatch with State::LEN"
        );

        // ASSERTION 2: Validate that the Borsh-serialized byte array has exactly the same
        // size and internal layout (including explicit padding) as the #[repr(C)]
        // struct would have in memory.
        assert_eq!(
            bytes.len(),
            size_of::<State>(),
            "Borsh serialized length mismatch with in-memory size_of::<State>() - \
             This indicates a discrepancy between Borsh's packing and #[repr(C)]'s layout, \
             likely due to incorrect explicit padding."
        );

        // 3. Unsafe reinterpretation: Take the Borsh-serialized bytes and
        // directly cast them to a reference to `State`.
        // This is safe ONLY because the previous assertion guarantees the byte
        // layout matches.
        let actual = unsafe {
            let ptr = bytes.as_ptr() as *const State;
            &*ptr
        };

        // 4. Assert that all fields (including padding) match.
        assert_eq!(actual.discriminator, expected.discriminator);
        assert_eq!(actual.padding1, expected.padding1, "Padding1 mismatch");
        assert_eq!(actual.escrowed_orca_amount, expected.escrowed_orca_amount);
        assert_eq!(actual.cool_down_period_s, expected.cool_down_period_s);
        assert_eq!(actual.update_authority, expected.update_authority);
        assert_eq!(actual.padding2, expected.padding2, "Padding2 mismatch");

        // 5. Sanity check: Ensure standard Borsh deserialization also works as expected.
        let deserialized_state = State::try_from_slice(&bytes).unwrap();
        assert_eq!(deserialized_state, expected);
    }

    #[test]
    fn test_state_calculated_sizes() {
        // 1. Calculate the expected size of the core data fields
        //    (excluding final padding2, but including padding1)
        let core_data_with_internal_padding_size: usize = size_of::<AccountDiscriminator>() // 1 byte
            + size_of::<[u8; 6]>() // 6 bytes (padding1)
            + size_of::<u8>() // 1 byte (bump)
            + size_of::<u64>() // 8 bytes
            + size_of::<u64>() // 8 bytes
            + size_of::<Pubkey>(); // 32 bytes

        // Expected sum: 1 + 7 + 8 + 8 + 32 = 56 bytes
        assert_eq!(core_data_with_internal_padding_size, 56);

        let total_calculated_struct_size =
            core_data_with_internal_padding_size + size_of::<[u8; 1992]>();

        assert_eq!(total_calculated_struct_size, STATE_ACCOUNT_LEN);
        assert_eq!(size_of::<State>(), STATE_ACCOUNT_LEN);
        assert_eq!(size_of::<State>(), total_calculated_struct_size);
    }
}
