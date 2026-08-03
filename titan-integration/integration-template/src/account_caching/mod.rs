pub mod rpc_cache;

use solana_account::Account;
use thiserror::Error;

use async_trait::async_trait;
use solana_pubkey::Pubkey;

/// Trait that abstracts account retrieval for Titan.
///
/// A venue receives an `&dyn AccountsCache` during `update_state()` and uses it
/// to retrieve all required on-chain accounts.  
///
/// Implementers must satisfy the following:
///
/// - **Thread-safety:** Trait objects must be `Send + Sync`.
/// - **Deterministic ordering:** `get_accounts()` must return accounts in the
///   same order as the input pubkeys.
/// - **Graceful failure:** Missing accounts should trigger structured errors.
/// - **Caching optional:** The trait *allows* caching but does not require it.
///
/// Typical implementations include:
/// - `RpcClientCache` (network-backed, concurrent cache)
/// - Test harnesses using fixed HashMaps
/// - Simulators like LiteSVM or custom in-process banks
#[async_trait]
pub trait AccountsCache: Send + Sync {
    /// Retrieve a single account by pubkey.
    ///
    /// Returns:
    /// - `Ok(Some(Account))` if the account exists
    /// - `Ok(None)` if the account is known to be missing
    ///
    /// Errors:
    /// - RPC failures
    /// - Lock acquisition failures
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>, AccountCacheError>;

    /// Retrieve multiple accounts in a single operation.
    ///
    /// The returned vector must have the same length and ordering as `pubkeys`.
    ///
    /// Returns:
    /// - `Ok(vec![Some(acc1), None, Some(acc3), ...])`
    ///
    /// Errors:
    /// - RPC or batching failures
    /// - Any unexpected missing account (if implementation cannot detect)
    async fn get_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, AccountCacheError>;
}

/// Errors that may occur when using `AccountsCache`.
///
/// These are *not* AMM or venue-level errors. They strictly represent errors
/// from the account-fetching layer.
///
/// Variants include:
/// - Missing accounts
/// - RPC client errors
/// - Concurrency/locking problems
/// - Crossbeam channel errors (for caches using multi-threaded pipelines)
#[derive(Debug, Error)]
pub enum AccountCacheError {
    /// RPC client failed to fetch account data.
    ///
    /// Automatically converted from `ClientError` through `#[from]`.
    #[error("Failed to fetch account")]
    FailedToFetchAccount(#[from] solana_client::client_error::ClientError),

    /// Failure acquiring a write lock, if the cache uses lock-based concurrency.
    #[error("Failed to acquire write lock")]
    FailedToAcquireWriteLock,

    /// Failure acquiring a read lock.
    #[error("Failed to acquire read lock")]
    FailedToAcquireReadLock,
}

/// Ensures `AccountCacheError` satisfies `Send + Sync` at compile time.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AccountCacheError>();
};
