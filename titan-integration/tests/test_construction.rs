mod common;

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use common::init_test_logger;
use titan_integration_template::trading_venue::{QuoteRequest, SwapType, TradingVenue};
use tracing::debug;

use crate::common::{VenueContext, build_venue_context};

#[cfg(debug_assertions)] // required when disable_release is set (default)
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

#[tokio::test]
async fn test_construction() {
    init_test_logger();

    //
    // Fetch the venue’s account and construct the venue
    //
    let VenueContext { venue, .. }: VenueContext = build_venue_context().await;
    //
    // Validate token metadata
    //
    // Only ORCA -> xORCA swap supported through stake operation.
    // (Unstake is not immediate, and cannot be done in a single transaction.)
    //
    let input_idx = 0;
    let output_idx = 1;
    let token_info = venue.get_token_info();
    debug!("Loaded token info: {:#?}", token_info);
    assert!(!token_info.is_empty());

    //
    // Validate quoting boundaries and quote correctness.
    //
    debug!("Checking bounds for pair ({}, {})", input_idx, output_idx);
    let (lower_bound, upper_bound) =
        assert_no_alloc(|| venue.bounds(input_idx, output_idx)).expect("Boundary search failed");
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
    debug!("Lower-bound quote: {:#?}", lb_result);
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
    debug!("Upper-bound quote: {:#?}", ub_result);
    assert!(
        !ub_result.not_enough_liquidity,
        "Upper bound indicates insufficient liquidity"
    );
    assert!(
        ub_result.expected_output > 0,
        "Upper bound produced zero output"
    );
}
