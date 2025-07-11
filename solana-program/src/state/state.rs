use super::{AccountDiscriminator, ProgramAccount, DEFAULT_ACCOUNT_LEN};
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct State {
    pub discriminator: AccountDiscriminator, // 1 byte
    // Explicit padding to ensure that the next field (u64) is 8-byte aligned
    // in memory when #[repr(C)] is used.
    // Calculation: 8 (desired alignment) - 1 (discriminator size) = 7 bytes.
    pub padding1: [u8; 7],
    pub escrowed_orca_amount: u64, // 8 bytes
    pub cool_down_period_s: u64,   // 8 bytes
    pub update_authority: Pubkey,  // 32 bytes
    // DEFAULT_ACCOUNT_LEN (2048 bytes) - 56 = 1992 bytes.
    pub padding2: [u8; 1992],
}

impl Default for State {
    fn default() -> Self {
        Self {
            discriminator: AccountDiscriminator::State,
            padding1: [0; 7],
            escrowed_orca_amount: 0,
            update_authority: Pubkey::default(),
            cool_down_period_s: 0,
            padding2: [0; 1992],
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
    use std::mem::size_of;

    #[test]
    fn test_state_byte_alignment() {
        // Use distinct non-zero values for fields and padding to ensure all bytes
        // are correctly serialized/deserialized and reinterpreted.
        let expected = State {
            discriminator: AccountDiscriminator::State,
            padding1: [0xAA; 7],
            escrowed_orca_amount: 0x1122334455667788,
            cool_down_period_s: 0xAABBCCDDEEFF0011,
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
            + size_of::<[u8; 7]>() // 7 bytes (padding1)
            + size_of::<u64>() // 8 bytes
            + size_of::<u64>() // 8 bytes
            + size_of::<Pubkey>(); // 32 bytes

        // Expected sum: 1 + 7 + 8 + 8 + 32 = 56 bytes
        assert_eq!(core_data_with_internal_padding_size, 56);

        let total_calculated_struct_size =
            core_data_with_internal_padding_size + size_of::<[u8; 1992]>();

        assert_eq!(total_calculated_struct_size, DEFAULT_ACCOUNT_LEN);
        assert_eq!(size_of::<State>(), DEFAULT_ACCOUNT_LEN);
        assert_eq!(size_of::<State>(), total_calculated_struct_size);
    }
}
