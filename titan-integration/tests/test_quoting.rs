mod common;
mod simulation_utils;

#[cfg(test)]
mod simulations {
    //! Quoting tests for Titan-compatible AMM venues.
    //!
    //! The tests ensure:
    //! - The venue loads on-chain state correctly
    //! - It exposes valid token info
    //! - It establishes valid quoting boundaries for both swap directions
    //! - Its off-chain quote matches on-chain execution on and off the boundaries
    //! - Its quoting speed is sufficient for integration
    //!
    //! Any AMM integrator must pass these quoting tests to ensure their pool
    //! is safe, consistent, and suitable for Titan routing.

    use rstest::rstest;
    use std::time::Instant;
    use titan_integration_template::trading_venue::QuoteRequest;
    use titan_integration_template::trading_venue::SwapType;
    use titan_integration_template::trading_venue::TradingVenue;
    use tracing::debug;

    use crate::common::VenueContext;
    use crate::simulation_utils::SimulationContext;
    use crate::simulation_utils::sync_litesvm_clock;

    use super::common::{build_venue_context, init_test_logger};
    use super::simulation_utils::{sample_log_uniform_u64, setup_litesvm, sim_quote_request};

    // -------------------------------------------------------------------------
    // Test 1: check boundary values in simulation
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_bound_simulation() {
        init_test_logger();

        let VenueContext { venue, cache }: VenueContext = build_venue_context().await;

        //
        // Validate token metadata
        //
        let token_info = venue.get_token_info();
        debug!("Loaded token info: {:#?}", token_info);
        assert_eq!(token_info.len(), 2);

        // Setup simulation VM
        let SimulationContext {
            mut litesvm,
            keypair,
        } = setup_litesvm();

        // Sync sysvar clock to real network
        sync_litesvm_clock(&mut litesvm, &cache).await;

        //
        // For ORCA -> xORCA swap only, verify that boundary quotes match simulation.
        //
        let in_idx = 0;
        let out_idx = 1;
        let (lower, upper) = venue.bounds(in_idx as u8, out_idx as u8).unwrap();

        for bound in [lower, upper] {
            let request = QuoteRequest {
                input_mint: venue.get_token(in_idx).unwrap().pubkey,
                output_mint: venue.get_token(out_idx).unwrap().pubkey,
                amount: bound,
                swap_type: SwapType::ExactIn,
            };
            let sim_output =
                sim_quote_request(&venue, &cache, request.clone(), &mut litesvm, &keypair).await;
            let quote = venue.quote(request).unwrap();
            debug!(
                "Boundary = {}\nSimulated = {}\nOff-chain quote = {}\nDelta = {}",
                bound,
                sim_output,
                quote.expected_output,
                quote.expected_output.abs_diff(sim_output)
            );
            assert_eq!(quote.expected_output.abs_diff(sim_output), 0)
        }
    }

    // -------------------------------------------------------------------------
    // Test 2: Random sampling simulation
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_random_samples() {
        init_test_logger();

        let VenueContext { venue, cache }: VenueContext = build_venue_context().await;

        //
        // Validate token metadata
        //
        let token_info = venue.get_token_info();
        debug!("Loaded token info: {:#?}", token_info);
        assert_eq!(token_info.len(), 2);

        // Setup simulation VM
        let SimulationContext {
            mut litesvm,
            keypair,
        } = setup_litesvm();

        // Sync sysvar clock
        sync_litesvm_clock(&mut litesvm, &cache).await;

        //
        // For ORCA -> xORCA swap only, randomly sample the entire valid quoting domain and
        // ensure that the quoted amount matches the simulated amount.
        //
        let in_idx = 0;
        let out_idx = 1;
        let (lb, ub) = venue.bounds(in_idx, out_idx).unwrap();
        for _ in 0..50 {
            let amount = sample_log_uniform_u64(lb, ub);
            let request = QuoteRequest {
                input_mint: venue.get_token(in_idx as usize).unwrap().pubkey,
                output_mint: venue.get_token(out_idx as usize).unwrap().pubkey,
                amount,
                swap_type: SwapType::ExactIn,
            };
            let sim_output =
                sim_quote_request(&venue, &cache, request.clone(), &mut litesvm, &keypair).await;
            let quote = venue.quote(request).unwrap();
            debug!(
                "Random sim_output: {}\nQuote: {}\nDelta: {}",
                sim_output,
                quote.expected_output,
                quote.expected_output.abs_diff(sim_output)
            );
            assert_eq!(quote.expected_output.abs_diff(sim_output), 0)
        }
    }

    // -------------------------------------------------------------------------
    // Test 3: AMM Monotonicity
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_monotone() -> () {
        init_test_logger();

        let VenueContext { venue, .. }: VenueContext = build_venue_context().await;

        //
        // Validate token metadata
        //
        let token_info = venue.get_token_info();
        debug!("Loaded token info: {:#?}", token_info);
        assert_eq!(token_info.len(), 2);

        //
        // For ORCA -> xORCA swap only, verify that the swap function is monotone increasing.
        //
        let in_idx = 0;
        let out_idx = 1;
        let (lb, ub) = venue.bounds(in_idx, out_idx).unwrap();
        let mut test_amounts = Vec::with_capacity(50);

        for _ in 0..50 {
            test_amounts.push(sample_log_uniform_u64(lb, ub));
        }
        test_amounts.sort();

        let mut prev = 0;
        for amount in test_amounts {
            let input_mint = token_info[in_idx as usize].pubkey;
            let output_mint = token_info[out_idx as usize].pubkey;

            let result = venue
                .quote(QuoteRequest {
                    input_mint,
                    output_mint,
                    amount: amount,
                    swap_type: SwapType::ExactIn,
                })
                .expect("Lower-bound quote failed");

            debug!("quote: {:#?}", result);

            assert!(
                prev <= result.expected_output,
                "Swap function is not monotone (prev: {}) > (output: {})",
                prev,
                result.expected_output
            );

            prev = result.expected_output;
        }
    }

    // -------------------------------------------------------------------------
    // Test 4: Quoting speed
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    #[case(10_000)]
    async fn test_quoting_speed(#[case] iterations: usize) -> () {
        init_test_logger();

        let VenueContext { venue, .. }: VenueContext = build_venue_context().await;

        //
        // Validate token metadata
        //
        let token_info = venue.get_token_info();
        debug!("Loaded token info: {:#?}", token_info);
        assert_eq!(token_info.len(), 2);

        //
        // For ORCA -> xORCA swap only, verify quoting speed requirements are met.
        //
        let in_idx = 0;
        let out_idx = 1;
        let input_mint = token_info[in_idx as usize].pubkey;
        let output_mint = token_info[out_idx as usize].pubkey;

        let (lb, ub) = venue.bounds(in_idx, out_idx).unwrap();
        let mut test_amounts = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            test_amounts.push(sample_log_uniform_u64(lb, ub));
        }

        let start = Instant::now();
        for amount in test_amounts {
            let result = venue
                .quote(QuoteRequest {
                    input_mint,
                    output_mint,
                    amount: amount,
                    swap_type: SwapType::ExactIn,
                })
                .expect("Lower-bound quote failed");

            debug!("quote: {:#?}", result);
        }
        let elapsed = start.elapsed().as_secs_f64();
        let avg_time = elapsed / iterations as f64;

        debug!("Average quoting speed: {}", avg_time);

        assert!(
            avg_time < 0.0001,
            "Failed quoting speed test swapping ({}) -> ({})",
            input_mint,
            output_mint
        );
    }
}
