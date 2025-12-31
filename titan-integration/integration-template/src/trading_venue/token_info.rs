//! Token metadata and helpers used by Titan-compatible trading venues.
//!
//! `TokenInfo` encapsulates mint-level metadata for SPL Token and SPL Token
//! 2022 mints, including decimals, transfer-fee configuration, and whether the
//! mint belongs to the Token-2022 program. Titan venues use this metadata to
//! compute swap quotes correctly, construct ATAs, and apply fee-aware routing.

use solana_account::Account;
use solana_pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::solana_program::clock::Epoch;
use spl_token_2022::{
    extension::{BaseStateWithExtensions, StateWithExtensions},
    state::Mint,
};

use crate::trading_venue::error::TradingVenueError;

/// Canonical program ID for standard SPL Token.
pub const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;

/// Canonical program ID for the SPL Token 2022 program.
pub const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022::ID;

/// Representation of a token/mint used by a Titan-integrated venue.
///
/// This includes metadata derived from the mint account such as:
///
/// - The mint's public key  
/// - Decimals (UI precision)  
/// - Whether the mint uses Token-2022  
/// - Optional transfer-fee configuration  
///
/// Titan venues expose an array/slice of `TokenInfo` to describe the tokens
/// they support on a per-pool basis.
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenInfo {
    /// Mint address of the SPL token.
    pub pubkey: Pubkey,

    /// Number of decimal places a UI token amount should be scaled by.
    pub decimals: i32,

    /// `true` if the mint was issued under the Token-2022 program.
    pub is_token_2022: bool,

    /// Transfer-fee basis points (if the mint includes a TransferFee extension).
    ///  
    /// Value is in **basis points**, i.e. 1/100th of a percent.
    ///  
    /// Example: 25 → 0.25% fee.
    pub transfer_fee: Option<u16>,

    /// Maximum fee (in atom units) that may be applied per transfer.
    pub maximum_fee: Option<u64>,
}

impl TokenInfo {
    /// Construct `TokenInfo` by unpacking a mint account.
    ///
    /// This parses both SPL Token and Token-2022 mints. If the mint includes
    /// a Token-2022 transfer-fee extension, it is extracted and stored; otherwise
    /// `transfer_fee` and `maximum_fee` remain `None`.
    ///
    /// # Arguments
    /// - `pubkey` — the mint address  
    /// - `account` — the raw Solana account containing mint data  
    /// - `epoch` — the current epoch, used for fetching epoch-indexed fee values  
    ///
    /// # Errors
    /// Returns `TradingVenueError::DeserializationFailed` if the mint account
    /// cannot be decoded or is not a valid SPL mint.
    pub fn new(
        pubkey: &Pubkey,
        account: &Account,
        epoch: Epoch,
    ) -> Result<Self, TradingVenueError> {
        if let Ok(mint) = StateWithExtensions::<Mint>::unpack(&account.data) {
            let transfer_fee: Option<u16>;
            let maximum_fee: Option<u64>;

            // Extract Token-2022 transfer fee configuration if available.
            (transfer_fee, maximum_fee) =
                match mint
                    .get_extension::<spl_token_2022::extension::transfer_fee::TransferFeeConfig>()
                {
                    Ok(val) => {
                        let current_fees = val.get_epoch_fee(epoch);

                        // Convert to convenient primitive types.
                        let fee_bps: u16 = current_fees.transfer_fee_basis_points.into();

                        (Some(fee_bps), Some(current_fees.maximum_fee.into()))
                    }
                    Err(_) => (None, None),
                };

            Ok(TokenInfo {
                pubkey: *pubkey,
                decimals: mint.base.decimals as i32,
                is_token_2022: account.owner == spl_token_2022::ID,
                transfer_fee,
                maximum_fee,
            })
        } else {
            Err(TradingVenueError::DeserializationFailed(pubkey.into()))
        }
    }

    /// Return the SPL token program ID appropriate for this mint.
    ///
    /// - If `is_token_2022 == true`, returns `TOKEN_2022_PROGRAM_ID`
    /// - Otherwise returns the standard SPL `TOKEN_PROGRAM_ID`
    pub fn get_token_program(&self) -> Pubkey {
        if self.is_token_2022 {
            TOKEN_2022_PROGRAM_ID
        } else {
            TOKEN_PROGRAM_ID
        }
    }

    /// Compute the associated token account (ATA) address for the given wallet,
    /// using the correct token program ID (either Token or Token-2022).
    ///
    /// This ensures ATA derivation works for both program versions, since they
    /// do **not** share the same program ID.
    pub fn get_associated_token_address(&self, wallet_address: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(
            wallet_address,
            &self.pubkey,
            &self.get_token_program(),
        )
    }
}
