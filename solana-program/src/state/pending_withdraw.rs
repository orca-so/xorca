use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

const PENDING_WITHDRAW_LEN: usize = 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct PendingWithdraw {
    pub discriminator: AccountDiscriminator, // 1 byte
    // Explicit padding to ensure that the next field (u64) is 8-byte aligned
    // in memory when #[repr(C)] is used.
    // Calculation: 8 (desired alignment) - 1 (discriminator size) = 7 bytes.
    pub padding1: [u8; 7],
    pub withdrawable_orca_amount: u64, // 8 bytes
    pub withdrawable_timestamp: i64,   // 8 bytes
    // Remaining bytes to fill PENDING_WITHDRAW_LEN
    // Calculation: PENDING_WITHDRAW_LEN - (1 + 7 + 8 + 8) = 1024 - 24 = 1000 bytes.
    pub padding2: [u8; 1000],
}

impl Default for PendingWithdraw {
    fn default() -> Self {
        Self {
            discriminator: AccountDiscriminator::PendingWithdraw,
            padding1: [0; 7],
            withdrawable_orca_amount: 0,
            withdrawable_timestamp: 0,
            padding2: [0; 1000],
        }
    }
}

impl PendingWithdraw {
    pub fn seeds<'a>(unstaker: &'a Pubkey, withdraw_index: &'a [u8]) -> Vec<Seed<'a>> {
        vec![
            Seed::from(b"pending_withdraw"),
            Seed::from(unstaker),
            Seed::from(withdraw_index),
        ]
    }
}

impl ProgramAccount for PendingWithdraw {
    const LEN: usize = PENDING_WITHDRAW_LEN;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::PendingWithdraw;
}
#[cfg(test)]
mod tests {
    use super::*;
    use borsh::BorshDeserialize;
    use std::mem::size_of;

    #[test]
    fn test_pending_withdraw_byte_alignment() {
        // Use distinct non-zero values for fields and padding to ensure all bytes
        // are correctly serialized/deserialized and reinterpreted.
        let expected = PendingWithdraw {
            discriminator: AccountDiscriminator::PendingWithdraw,
            padding1: [0xAA; 7],
            withdrawable_orca_amount: 0x1122334455667788,
            // Corrected: Use a valid i64 hexadecimal literal
            withdrawable_timestamp: 0x0123456789ABCDEF, // Fits within i64
            padding2: [0xCC; 1000],
        };

        // 1. Serialize the struct using Borsh.
        // Because we added explicit padding fields that match #[repr(C)]'s
        // expected padding, the Borsh output will now have these padding bytes.
        let bytes = borsh::to_vec(&expected).unwrap();

        // ASSERTION 1: Check if the Borsh-serialized length matches the expected LEN.
        assert_eq!(
            bytes.len(),
            PendingWithdraw::LEN,
            "Borsh serialized length mismatch with PendingWithdraw::LEN"
        );

        // ASSERTION 2: Validate that the Borsh-serialized byte array has exactly the same
        // size and internal layout (including explicit padding) as the #[repr(C)]
        // struct would have in memory.
        assert_eq!(
            bytes.len(),
            size_of::<PendingWithdraw>(),
            "Borsh serialized length mismatch with in-memory size_of::<PendingWithdraw>() - \
             This indicates a discrepancy between Borsh's packing and #[repr(C)]'s layout, \
             likely due to incorrect explicit padding."
        );

        // 3. Unsafe reinterpretation: Take the Borsh-serialized bytes and
        // directly cast them to a reference to `PendingWithdraw`.
        // This is safe ONLY because the previous assertion guarantees the byte
        // layout matches.
        let actual = unsafe {
            let ptr = bytes.as_ptr() as *const PendingWithdraw;
            &*ptr
        };

        // 4. Assert that all fields (including padding) match.
        assert_eq!(actual.discriminator, expected.discriminator);
        assert_eq!(actual.padding1, expected.padding1, "Padding1 mismatch");
        assert_eq!(
            actual.withdrawable_orca_amount,
            expected.withdrawable_orca_amount
        );
        assert_eq!(
            actual.withdrawable_timestamp,
            expected.withdrawable_timestamp
        );
        assert_eq!(actual.padding2, expected.padding2, "Padding2 mismatch");

        // 5. Sanity check: Ensure standard Borsh deserialization also works as expected.
        let deserialized_state = PendingWithdraw::try_from_slice(&bytes).unwrap();
        assert_eq!(deserialized_state, expected);
    }

    #[test]
    fn test_pending_withdraw_calculated_sizes() {
        let core_data_with_internal_padding_size: usize = size_of::<AccountDiscriminator>() // 1 byte
            + size_of::<[u8; 7]>() // 7 bytes (padding1)
            + size_of::<u64>() // 8 bytes
            + size_of::<i64>(); // 8 bytes
        assert_eq!(core_data_with_internal_padding_size, 24);
        let total_calculated_struct_size =
            core_data_with_internal_padding_size + size_of::<[u8; 1000]>();
        assert_eq!(total_calculated_struct_size, PENDING_WITHDRAW_LEN);
        assert_eq!(size_of::<PendingWithdraw>(), PENDING_WITHDRAW_LEN);
        assert_eq!(size_of::<PendingWithdraw>(), total_calculated_struct_size);
    }
}
