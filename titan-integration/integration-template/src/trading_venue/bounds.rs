//! Boundary-finding utilities for Titan venues.
//!
//! Titan requires every venue to provide valid quoting ranges for a given
//! input/output token pair. These utilities search for the *maximal valid
//! quoting range* by probing the venue's `quote()` function.
//!
//! The boundary search consists of two phases:
//!
//! 1. **Coarse exponential search** (`find_boundaries_coarse`)  
//!    Finds rough intervals where quoting transitions from:
//!       - INVALID → VALID  (lower boundary)
//!       - VALID   → INVALID (upper boundary)
//!    using exponential stepping with overflow protection.
//!
//! 2. **Binary refinement** (`refine_lower`, `refine_upper`)  
//!    Narrows those rough intervals into precise boundaries.
//!
//! A quote is considered *valid* when:
//! - The venue returns `Ok(QuoteResult)`
//! - `not_enough_liquidity == false`
//! - `expected_output > 0`
//!
//! This module is protocol-agnostic and works for any Titan-integrated AMM.

use std::u64;

use crate::trading_venue::{QuoteResult, error::TradingVenueError};

/// Each step in exponential search is scaled by this factor.
const SCALING_FACTOR: u64 = 2;

/// Returns `true` if a quote is considered usable for routing.
///
/// A quote is invalid if:
/// - It reports insufficient liquidity (`not_enough_liquidity == true`)
/// - The output is zero (pool cannot execute a meaningful swap)
fn valid_quote(quote: &QuoteResult) -> bool {
    !(quote.not_enough_liquidity || quote.expected_output == 0)
}

/// Perform a **coarse exponential search** to determine an initial interval
/// around the valid quoting region.
///
/// Returns a 4-tuple of bounds:
///
/// ```text
/// (lower_low, lower_high, upper_low, upper_high)
/// ```
///
/// Where:
///
/// - `(lower_low → lower_high)` brackets the **first valid quote**  
///   i.e., lower_low is invalid, lower_high is valid.
/// - `(upper_low → upper_high)` brackets the **first invalid quote after the valid range**  
///   i.e., upper_low is valid, upper_high is invalid.
///
/// The returned coarse bounds are later refined by binary search.
///
/// # Behavior
/// - Starts probing from 1, doubling until a valid quote is found.
/// - Then continues doubling until the quotes become invalid again.
/// - Protects against `u64::MAX` and arithmetic overflow.
///
/// # Errors
/// Never returns an error directly, but the coarse search stops early when
/// quoting fails unexpectedly.
pub fn find_boundaries_coarse(
    f: &impl Fn(u64) -> Result<QuoteResult, TradingVenueError>,
) -> Result<(u64, u64, u64, u64), TradingVenueError> {
    // --- Phase 1: Find first valid quote ---
    let mut lower_low = 0;
    let mut lower_high = 1;

    // Expand until we find a valid quote.
    while {
        match f(lower_high) {
            Ok(result) if !valid_quote(&result) => true, // keep searching
            Ok(_result) => false,                        // found valid region
            Err(_) => true,                              // treat errors as invalid
        }
    } {
        lower_low = lower_high;
        lower_high = lower_high.saturating_mul(SCALING_FACTOR);

        // Overflow/saturation protection
        if lower_high <= lower_low || lower_high == u64::MAX {
            log::error!("Invalid lower/upper combination or hit u64::MAX");
            lower_high = u64::MAX;
            break;
        }
    }

    // --- Phase 2: Find first *invalid* quote above the valid region ---
    let mut upper_low = lower_high;
    let mut upper_high = upper_low.saturating_mul(SCALING_FACTOR);

    if upper_high <= upper_low {
        // Overflow from previous step
        upper_high = upper_low;
    } else {
        while let Ok(result) = f(upper_high) {
            if !valid_quote(&result) {
                break;
            }

            upper_low = upper_high;
            upper_high = upper_high.saturating_mul(SCALING_FACTOR);

            if upper_high <= upper_low || upper_high == u64::MAX {
                log::trace!("Hit overflow during upper search");
                upper_high = u64::MAX;
                break;
            }
        }
    }

    Ok((lower_low, lower_high, upper_low, upper_high))
}

/// Refine the lower boundary via binary search.
///
/// Given an interval:
///
/// ```text
/// low  (invalid) → high (valid)
/// ```
///
/// This function performs binary search to find the *smallest* value that
/// produces a valid quote.
///
/// Searches until the interval is narrower than ~100 atoms.
///
/// # Errors
/// Only returns errors from the provided quoting function.
pub fn refine_lower(
    f: &impl Fn(u64) -> Result<QuoteResult, TradingVenueError>,
    mut low: u64,
    mut high: u64,
) -> Result<u64, TradingVenueError> {
    // These invariant checks should normally never trigger.
    let low_quote = f(low);
    let high_quote = f(high);

    if let Ok(ref result) = low_quote {
        if valid_quote(result) {
            log::error!(
                "The lower low quotes successfully; this contradicts the search invariant."
            );
        }
    }

    match high_quote {
        Ok(result) => {
            if !valid_quote(&result) {
                log::error!("The upper low is invalid; this contradicts the search invariant.");
            }
        }
        Err(e) => {
            log::error!(
                "The upper low errored; this contradicts the search invariant: {:?}",
                e
            );
        }
    }

    // Binary search
    while (high - low) > 100 {
        let mid = high / 2 + low / 2;

        match f(mid) {
            Ok(result) => {
                if valid_quote(&result) {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            Err(_) => low = mid,
        }
    }

    Ok(high)
}

/// Refine the upper boundary via binary search.
///
/// Given an interval:
///
/// ```text
/// low (valid) → high (invalid)
/// ```
///
/// This function finds the *largest* value that still produces a valid quote.
///
/// Searches until the interval is narrower than ~100 atoms.
///
/// # Errors
/// Only returns errors from the provided quoting function.
pub fn refine_upper(
    f: &impl Fn(u64) -> Result<QuoteResult, TradingVenueError>,
    mut low: u64,
    mut high: u64,
) -> Result<u64, TradingVenueError> {
    let low_quote = f(low);
    let high_quote = f(high);

    // Sanity checks ― not usually hit
    match low_quote {
        Ok(result) => {
            if !valid_quote(&result) {
                log::error!("The upper low is invalid; this contradicts invariants.");
            }
        }
        Err(e) => {
            log::error!(
                "The upper low errored; this contradicts invariants: {:?}",
                e
            );
        }
    }

    if let Ok(ref result) = high_quote {
        if valid_quote(result) && high != u64::MAX {
            log::error!("The upper high is valid; this contradicts the expected invalid boundary.");
        }
    }

    // Binary search
    while (high - low) > 100 {
        let mid = high / 2 + low / 2;

        match f(mid) {
            Ok(result) => {
                if valid_quote(&result) {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            Err(_) => high = mid,
        }
    }

    Ok(low)
}

/// Unified boundary search.
/// Returns `(lower_bound, upper_bound)` such that:
///
/// - For all `x < lower_bound`, quoting is invalid  
/// - For all `lower_bound ≤ x ≤ upper_bound`, quoting is valid  
/// - For all `x > upper_bound`, quoting is invalid
///
/// The returned interval represents the **maximal valid input range** for the
/// given pool and token pair.
///
/// # Errors
/// - `BoundarySearchFailed` if the search collapses to a degenerate interval  
/// - `NoQuotableValue` if no valid quote exists at any input (pool unusable)
pub fn find_boundaries(
    f: &impl Fn(u64) -> Result<QuoteResult, TradingVenueError>,
) -> Result<(u64, u64), TradingVenueError> {
    let (lower_low, lower_high, upper_low, upper_high) = find_boundaries_coarse(f)?;

    // Degenerate interval: the entire domain is invalid.
    if lower_low == upper_high {
        return Err(TradingVenueError::BoundarySearchFailed(
            "Search boundaries are all equal; search space collapsed".into(),
        ));
    }

    // Never found a valid quote
    if lower_high == u64::MAX {
        return Err(TradingVenueError::NoQuotableValue(
            "No quotable value found; exponential search hit u64::MAX".into(),
        ));
    }

    let lower_bound = refine_lower(f, lower_low, lower_high)?;
    let upper_bound = refine_upper(f, upper_low, upper_high)?;

    Ok((lower_bound, upper_bound))
}
