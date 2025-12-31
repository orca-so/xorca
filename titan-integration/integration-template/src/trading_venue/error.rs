//! Error types for Titan trading-venue integration.
//!
//! This module defines the canonical error model used throughout the Titan
//! venue abstraction layer. Any AMM, CLMM, orderbook, or proprietary pool
//! integrating with Titan must surface failures through `TradingVenueError`.
//!
//! The error types are intentionally granular and expressive, making them
//! suitable both for routing diagnostics and user-facing error messages.
//!
//! `ErrorInfo` is a lightweight container for attaching consistent metadata to
//! an error (e.g., a failing pubkey or message). Most variants use it so that
//! callers can render meaningful context without allocating dynamically.

use std::fmt::Display;

use solana_pubkey::Pubkey;
use thiserror::Error;

use crate::{account_caching::AccountCacheError, trading_venue::protocol::PoolProtocol};

/// Wrapper type for attaching additional context to an error.
///
/// Many venue failures are associated with a specific pubkey (e.g., missing
/// account, invalid mint) or with a simple string message. `ErrorInfo` keeps
/// all those representations in a unified, displayable type.
///
/// Variants:
/// - `Pubkey` — attach a specific Solana account address  
/// - `String` — arbitrary owned string describing the error  
/// - `StaticStr` — lightweight static string reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorInfo {
    Pubkey(Pubkey),
    String(String),
    StaticStr(&'static str),
}

impl From<Pubkey> for ErrorInfo {
    fn from(pubkey: Pubkey) -> Self {
        ErrorInfo::Pubkey(pubkey)
    }
}

impl From<&Pubkey> for ErrorInfo {
    fn from(pubkey: &Pubkey) -> Self {
        ErrorInfo::Pubkey(*pubkey)
    }
}

impl From<String> for ErrorInfo {
    fn from(string: String) -> Self {
        ErrorInfo::String(string)
    }
}

impl From<&'static str> for ErrorInfo {
    fn from(string: &'static str) -> Self {
        ErrorInfo::StaticStr(string)
    }
}

impl Display for ErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorInfo::Pubkey(pubkey) => write!(f, "{}", pubkey),
            ErrorInfo::String(string) => write!(f, "{}", string),
            ErrorInfo::StaticStr(string) => write!(f, "{}", string),
        }
    }
}

/// Errors that may occur during Titan’s venue initialization, state update,
/// quoting, or swap instruction generation.
///
/// Every venue implementer is expected to use these variants consistently.
/// Errors returned here are surfaced directly to Titan's router, logs, and
/// debugging interfaces, meaning that clear contextual errors are extremely
/// valuable.
///
/// Error variants fall into several categories:
///
/// **Account/State issues**  
/// - `NoAccountFound`  
/// - `FailedToFetchAccountData`  
/// - `FailedToFetchMultipleAccountData`  
/// - `DeserializationFailed`  
/// - `MissingState`  
/// - `NotInitialized`
///
/// **Mint / token issues**  
/// - `InvalidMint`  
/// - `TokenInfoIndexError`  
///
/// **Math issues**  
/// - `CheckedMathError`  
/// - `MathError`  
///
/// **Swap/venue behavior issues**  
/// - `AmmMethodError`  
/// - `ExactOutNotSupported`  
/// - `UnsupportedVenue`  
/// - `InactivePoolError`
///
/// **Boundary search & quoting issues**  
/// - `BoundarySearchFailed`  
/// - `NoQuotableValue`
///
/// **Internal/unexpected issues**  
/// - `SomethingWentWrong` (boxed error for unexpected failures)  
///
/// **Infrastructure issues**  
/// - `CacheUnlockFailed`  
/// - `AccountCacheError` (converted via `#[from]`)
#[derive(Error, Debug)]
pub enum TradingVenueError {
    /// No account exists in the RPC or cache for the given pubkey.
    #[error("No account found for pubkey: {0}")]
    NoAccountFound(ErrorInfo),

    /// Failed to construct a venue's internal state object from a Solana account.
    #[error("Unable to build venue from account for pubkey: {0}")]
    FromAccountError(ErrorInfo),

    /// Failed to concurrently fetch multiple accounts from RPC.
    #[error("Failed to fetch multiple account data")]
    FailedToFetchMultipleAccountData,

    /// Failed to fetch a single account from RPC.
    #[error("Failed to fetch account data: {0}")]
    FailedToFetchAccountData(ErrorInfo),

    /// Mint or pool account could not be deserialized into expected data structures.
    #[error("Failed to deserialize account data: {0}")]
    DeserializationFailed(ErrorInfo),

    /// Failed to obtain a lock on the account cache (unexpected threading issue).
    #[error("Failed to unlock cache")]
    CacheUnlockFailed,

    /// The venue has not yet loaded its required accounts.
    ///
    /// This usually indicates that `update_state()` was not called before
    /// attempting a quote or instruction build.
    #[error("The venue has not had its accounts loaded")]
    NotInitialized(ErrorInfo),

    /// A required field of the venue's state structure was not loaded or set.
    #[error("The state object is not loaded: {0}")]
    MissingState(ErrorInfo),

    /// Mint provided is invalid or mismatched for the venue.
    #[error("Invalid mint: {0}")]
    InvalidMint(ErrorInfo),

    /// Arithmetic performed via checked math failed (overflow, underflow, etc.).
    #[error("Checked math error: {0}")]
    CheckedMathError(ErrorInfo),

    /// Error returned from internal AMM logic.
    #[error("Error from method: {0}")]
    AmmMethodError(ErrorInfo),

    /// Venues must currently support `ExactIn`; this error is returned if a caller
    /// requests an unsupported `ExactOut` quote or swap.
    #[error("Exact Out swap type is not supported")]
    ExactOutNotSupported,

    /// Token amount / numeric conversion failure (e.g. atom scaling issues).
    #[error("Data conversion error: {0}")]
    DataConversionError(ErrorInfo),

    /// Error during boundary search where the search interval collapses or an
    /// invariant is violated.
    #[error("Boundary search failed: {0}")]
    BoundarySearchFailed(ErrorInfo),

    /// Boundary search found no valid quoting value at any tested amount.
    #[error("Boundary search failed: {0}")]
    NoQuotableValue(ErrorInfo),

    /// Catch-all wrapper for unexpected boxed errors.
    #[error("Something went wrong: {0}")]
    SomethingWentWrong(Box<dyn std::error::Error>),

    /// The venue protocol or configuration is unsupported by Titan.
    #[error("Unsupported venue: {0}")]
    UnsupportedVenue(ErrorInfo),

    /// A `TokenInfo` index was requested that does not exist in the venue's metadata.
    #[error("Token info does not extend to index {0}")]
    TokenInfoIndexError(usize),

    /// Miscellaneous math error not captured by checked math.
    #[error("Math Error: {0}")]
    MathError(ErrorInfo),

    /// Generic deserialization failure.
    #[error("Deserialization Error: {0}")]
    DeserializationError(ErrorInfo),

    /// A pool exists on-chain but is inactive or not usable for routing.
    #[error("Pool {0} from protocol {1} is inactive")]
    InactivePoolError(Pubkey, PoolProtocol),

    /// Error produced by Titan’s account cache layer.
    #[error("Account cache error: {0}")]
    AccountCacheError(#[from] AccountCacheError),
}
