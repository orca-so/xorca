#[cfg(test)]
mod test_construction {
    //! Integration test ensuring that a venue:
    //! - can be constructed from on-chain account data,
    //! - can load its required state via the AccountsCache,
    //! - returns valid token info,
    //! - supports quoting for both swap directions,
    //! - and exposes sane quoting boundaries.
    //!
    //! Any AMM implementer integrating with Titan should ensure their venue
    //! passes this style of test, as it verifies the critical invariants that
    //! Titan relies on for routing.

    use std::{env, str::FromStr};

    use rstest::rstest;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_pubkey::Pubkey;
    use titan_integration_template::account_caching::rpc_cache::RpcClientCache;
    use titan_integration_template::trading_venue::{QuoteRequest, SwapType};
    use titan_integration_template::{
        example::RaydiumAmmVenue,
        trading_venue::{FromAccount, TradingVenue},
    };

    use assert_no_alloc::*;

    #[cfg(debug_assertions)] // required when disable_release is set (default)
    #[global_allocator]
    static A: AllocDisabler = AllocDisabler;

    /// Initialize logging for test output.
    ///
    /// Having logging enabled is extremely helpful when debugging state-loading
    /// issues or boundary failures during venue development.
    fn init_test_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Ensure that the venue can:
    /// - Build from a raw on-chain account,
    /// - Perform a state update using the caching layer,
    /// - Report valid token metadata,
    /// - Calculate valid quoting boundaries,
    /// - Return nonzero, liquidity-supported quotes at both boundary edges.
    ///
    #[rstest]
    #[tokio::test]
    #[case("Bzc9NZfMqkXR6fz1DBph7BDf9BroyEf6pnzESP7v5iiw")] // Example Raydium pool
    async fn test_construction(#[case] amm_key: String) {
        init_test_logger();

        //
        // Prepare inputs
        //
        let amm_key = Pubkey::from_str(&amm_key).expect("Invalid test pubkey");

        let rpc_url =
            env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL must be set for integration tests");
        let rpc = RpcClient::new(rpc_url);

        //
        // Fetch the venue’s account and construct the venue
        //
        let venue_account = rpc
            .get_account(&amm_key)
            .await
            .expect("Failed to fetch AMM account");

        let mut venue = RaydiumAmmVenue::from_account(&amm_key, &venue_account)
            .expect("Failed to construct venue from account");

        //
        // Load on-chain state using the caching layer
        //
        let cache = RpcClientCache::new(rpc);
        venue
            .update_state(&cache)
            .await
            .expect("Venue state update failed");

        //
        // Validate token metadata
        //
        let token_info = venue.get_token_info();
        log::info!("Loaded token info: {:#?}", token_info);
        assert!(token_info.len() > 0);

        // Raydium AMMs always have 2 tokens.
        assert_eq!(token_info.len(), 2);

        //
        // 5. For each direction (token0 → token1, token1 → token0)
        //    validate quoting boundaries and quote correctness.
        //
        for (input_idx, output_idx) in [(0, 1), (1, 0)] {
            log::info!("Checking bounds for pair ({}, {})", input_idx, output_idx);

            let (lower_bound, upper_bound) =
                assert_no_alloc(|| venue.bounds(input_idx, output_idx))
                    .expect("Boundary search failed");

            assert!(
                lower_bound < upper_bound,
                "Lower bound must be strictly less than upper bound"
            );

            let input_mint = token_info[input_idx as usize].pubkey;
            let output_mint = token_info[output_idx as usize].pubkey;

            let lb_result = assert_no_alloc(|| {
                venue.quote(QuoteRequest {
                    input_mint,
                    output_mint,
                    amount: lower_bound,
                    swap_type: SwapType::ExactIn,
                })
            })
            .expect("Lower-bound quote failed");

            log::info!("Lower-bound quote: {:#?}", lb_result);

            assert!(
                !lb_result.not_enough_liquidity,
                "Lower bound indicates insufficient liquidity"
            );
            assert!(
                lb_result.expected_output > 0,
                "Lower bound produced zero output"
            );

            let ub_result = assert_no_alloc(|| {
                venue.quote(QuoteRequest {
                    input_mint,
                    output_mint,
                    amount: upper_bound,
                    swap_type: SwapType::ExactIn,
                })
            })
            .expect("Upper-bound quote failed");

            log::info!("Upper-bound quote: {:#?}", ub_result);

            assert!(
                !ub_result.not_enough_liquidity,
                "Upper bound indicates insufficient liquidity"
            );
            assert!(
                ub_result.expected_output > 0,
                "Upper bound produced zero output"
            );
        }
    }
}
