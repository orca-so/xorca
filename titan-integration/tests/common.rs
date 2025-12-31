use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::account::Account;
use titan_integration_template::{
    account_caching::{AccountsCache, rpc_cache::RpcClientCache},
    trading_venue::{FromAccount, TradingVenue},
};
use xorca_titan_integration::{amm::XOrcaVenue, constants::STATE_KEY};

pub fn init_test_logger() {
    let _ = tracing_subscriber::fmt::try_init();
}

pub fn rpc_client() -> RpcClient {
    let rpc_url =
        std::env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL must be set for integration tests");
    RpcClient::new(rpc_url)
}

pub async fn venue_account(cache: &dyn AccountsCache) -> Account {
    cache
        .get_account(&STATE_KEY)
        .await
        .expect("Failed to fetch xORCA state account")
        .unwrap()
}

pub struct VenueContext {
    pub venue: XOrcaVenue,
    pub cache: RpcClientCache,
}

pub async fn build_venue_context() -> VenueContext {
    let rpc: RpcClient = rpc_client();
    let cache = RpcClientCache::new(rpc);
    let venue_account = venue_account(&cache).await;
    let mut venue = XOrcaVenue::from_account(&STATE_KEY, &venue_account).unwrap();
    let required_pubkeys = venue
        .get_required_pubkeys_for_update()
        .expect("Failed to get required pubkeys for update");
    let _prefetched_accounts = cache
        .get_accounts(&required_pubkeys)
        .await
        .expect("Failed to get accounts");
    venue
        .update_state(&cache)
        .await
        .expect("Venue state update failed");
    VenueContext { venue, cache }
}
