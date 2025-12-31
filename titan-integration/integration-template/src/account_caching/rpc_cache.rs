//! An RPC-backed account cache for Titan venue state updates.
//!
//! `RpcClientCache` wraps a `RpcClient` and provides a thread-safe, in-memory
//! cache of Solana accounts. This accelerates venue state updates by avoiding
//! redundant RPC calls and ensures consistent account snapshots when integrating
//! multiple pools and venues.
//!
//! The cache implements the `AccountsCache` trait, which is used by Titan's
//! trading venues during:
//! - Pool initialization
//! - State refreshing
//! - Boundary scanning
//! - Quoting
//!
//! The internal storage uses a `DashMap<Pubkey, Option<Account>>`, making it
//! both concurrent and lock-free at the application level.

use ahash::AHashMap;
use async_trait::async_trait;
use dashmap::DashMap;
use solana_account::Account;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::account_caching::{AccountCacheError, AccountsCache};

/// Internal alias for the in-memory account cache.
/// Stores `Some(Account)` for found accounts and `None` for known-missing accounts.
///
/// Using `Option<Account>` avoids retrying missing accounts on every request.
type AccountCache = DashMap<Pubkey, Option<Account>>;

/// A caching layer around a Solana RPC client.
///
/// The cache performs the following optimizations:
///
/// - **Single-account fetch**: Cache hits avoid RPC calls entirely.
/// - **Multi-account fetch**: Groups unknown keys into a single `get_multiple_accounts` RPC call.
/// - **Caching negative lookups**: Accounts that consistently return `None` are also stored.
/// - **Thread-safe reads/writes** using `DashMap`.
pub struct RpcClientCache {
    rpc_client: RpcClient,
    cache: AccountCache,
}

impl RpcClientCache {
    /// Construct a new RPC cache from an existing `RpcClient`.
    pub fn new(rpc_client: RpcClient) -> Self {
        let cache = AccountCache::default();
        Self { rpc_client, cache }
    }

    /// Clear all cached entries.
    ///
    /// Useful when a system update or transaction batch invalidates local state.
    pub fn reset_cache(&mut self) {
        self.cache.clear();
    }

    /// Retrieve multiple accounts from the cache without making RPC requests.
    ///
    /// For each pubkey:
    /// - If present in the cache → returned immediately.
    /// - If absent → `None` is returned.
    ///
    /// This does **not** fetch from RPC; it only reads cached values.
    pub fn get_multiple(&self, pubkeys: &[Pubkey]) -> Vec<Option<Account>> {
        let mut result = Vec::with_capacity(pubkeys.len());
        pubkeys.iter().for_each(|key| {
            if let Some(value) = self.cache.get(key) {
                result.push(value.clone());
            } else {
                result.push(None);
            }
        });

        result
    }
}

#[async_trait]
impl AccountsCache for RpcClientCache {
    /// Get a single account by pubkey.
    ///
    /// - Cache hit → returned immediately.
    /// - Cache miss → RPC call made, then result cached.
    ///
    /// Errors are converted into `AccountCacheError`.
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>, AccountCacheError> {
        if let Some(account) = self.cache.get(pubkey) {
            return Ok(account.to_owned());
        }

        let response: Account = self
            .rpc_client
            .get_account(pubkey)
            .await
            .map_err(|e| AccountCacheError::FailedToFetchAccount(e))?;

        // Cache positive lookup
        self.cache.insert(*pubkey, Some(response.clone()));

        Ok(Some(response))
    }

    /// Fetch multiple accounts, using cached values where possible and batching
    /// missing keys into a single RPC call.
    ///
    /// Steps:
    /// 1. Split pubkeys into cache hits and misses.
    /// 2. Fetch misses using `get_multiple_accounts`.
    /// 3. Store results (including `None` values) in cache.
    /// 4. Return accounts in the same order as `pubkeys`.
    async fn get_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, AccountCacheError> {
        let mut keys = Vec::new();
        let mut result_map: AHashMap<Pubkey, Option<Account>> = AHashMap::default();
        let cached_results = self.get_multiple(pubkeys);

        // Identify cache hits and misses
        cached_results
            .iter()
            .zip(pubkeys.iter())
            .for_each(|(account, pubkey)| {
                if let Some(res) = account {
                    // Cached hit
                    result_map.insert(*pubkey, Some(res.clone()));
                } else {
                    // Needs RPC fetch
                    keys.push(*pubkey);
                }
            });

        // Batch RPC call for missing keys
        if !keys.is_empty() {
            let response = self
                .rpc_client
                .get_multiple_accounts(&keys)
                .await
                .map_err(|e| AccountCacheError::FailedToFetchAccount(e))?;

            // Update map and cache
            for (pubkey, account) in keys.iter().zip(response.iter()) {
                result_map.insert(*pubkey, account.clone());
                self.cache.insert(*pubkey, account.clone());
            }
        }

        // Reassemble results in original input order
        let mut result = Vec::new();

        for pubkey in pubkeys {
            let item = result_map.get(pubkey).unwrap();
            result.push(item.clone());
        }

        Ok(result)
    }
}
