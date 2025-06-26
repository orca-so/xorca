use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::account_info::{Ref, RefMut};
use shank::ShankType;
use strum::Display;

pub mod pending_withdraw;
pub mod xorca_state;

#[derive(
    Debug, Clone, Copy, BorshSerialize, BorshDeserialize, Display, ShankType, PartialEq, Eq,
)]
#[repr(u8)]
pub enum AccountDiscriminator {
    XorcaState,      // 0
    PendingWithdraw, // 1
}

// Program accounts must be bytemuck <> borsh interoperable. If repr(C) is used, the struct
// is aligned meaning that in certain places, we need to add explicity padding to ensure that
// the struct is byte-for-byte compatible with bytemuck and borsh.

pub trait ProgramAccount: BorshSerialize + BorshDeserialize {
    const LEN: usize;
    const DISCRIMINATOR: AccountDiscriminator;

    fn from_bytes(bytes: Ref<'_, [u8]>) -> Ref<'_, Self> {
        Ref::map(bytes, |bytes| unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    fn from_bytes_mut(bytes: RefMut<'_, [u8]>) -> RefMut<'_, Self> {
        RefMut::map(bytes, |bytes| unsafe {
            &mut *(bytes as *mut _ as *mut Self)
        })
    }
}
