# Titan AMM Integration Template

A reference implementation and test suite for integrating AMMs, CLMMs, and proprietary liquidity engines with Titan’s unified routing layer.

## Overview

Titan aggregates liquidity from heterogeneous venues (AMMs, CLMMs, orderbooks, proprietary pools) under a single unified quoting and routing interface.

This repository provides:

- A standard trait interface (TradingVenue) every venue must implement
- A robust boundary-search engine for computing safe swap-size ranges
- Token metadata utilities, including Token-2022 support
- A caching abstraction for efficient on-chain account loading
- Simulation tests using LiteSVM ensuring off-chain quotes match on-chain execution
- A fully worked Raydium example implementation

This template is the starting point for integrating your AMM into Titan.

## Core Components
#### 1. TradingVenue trait

This is the heart of integration. Your AMM implements:
```r
initialized()

from_account()

update_state()

get_token_info()

quote()

generate_swap_instruction()
```
Titan handles:

- Boundaries: `bounds()`
- Swap direction checks
-  Token metadata accessors

Assuming that your AMM admits a simplified method for finding
boundaries, you may provide this.

#### 2. QuoteRequest and QuoteResult

All quotes use raw atom units — no decimals.

#### 3. Boundary search

The module bounds.rs provides a robust search algorithm that:

- Discovers the maximal safe input range
- Applies exponential search + binary refinement
- Venues only need to implement a zero-input-safe quote().

#### 4. Token metadata (TokenInfo)

Supports:
- SPL Token
- Token-2022
- Transfer fee extensions

Do not include transfer fee handling in your quoting logic.

#### 5. AccountsCache

Used by venues to load their required on-chain accounts efficiently.
Includes an RPC-backed implementation with caching.

## Included Tests

This template ships with two categories of tests that every venue must pass:

#### 1. Construction & Boundary Tests

File: tests/test_construction.rs

This verifies that your venue correctly:

- Deserializes accounts
- Loads required state
- Produces valid token info
- Provides working quotes at both boundaries
- Does not allocate heap memory in quoting logic---quoting must be real-time

#### 2. Simulation Tests (Critical)

File: tests/simulations.rs

Uses LiteSVM to execute real swaps on your pool program.

It verifies:

- The off-chain quote() matches on-chain execution at boundaries
- Random log-uniform samples also match simulated execution
- ATA creation, token ownership, and instruction accounts are correct
- These tests ensure your AMM is safe for Titan’s routing engine.

## Implementing Your Own Venue

To integrate your pool:

Create a new struct
```r
pub struct MyCustomAmmVenue { ... }
```

Implement:
```r
impl FromAccount for MyCustomAmmVenue { ... }
impl TradingVenue for MyCustomAmmVenue { ... }
```

Required methods:
```r
from_account()
# Load initial pool metadata (e.g., tick array addresses, vaults, invariant configs)

update_state()
# Fetch required accounts via AccountsCache and deserialize live state
# (tick arrays, vault balances, etc.)

get_token_info()
# Return token mints + decimals + token programs

quote()
# Perform off-chain math for your AMM and return QuoteResult

generate_swap_instruction()
# Construct your on-chain program's swap IX for a user
```
Run the included tests and ensure they pass:
```bash
cargo test -- --nocapture
```

If your venue passes the tests, your quote() logic is sufficient
to be assessed by our team and go through the next stages of
integration.

## Tips for Integrators
1. Always support zero-input quoting
2. Keep your deserialization strictly defensive, never panic
3. Don’t perform I/O, allocate heap memory, or panic inside quote()
4. It should take significantly less than 0.1ms to quote your venue
5. Make sure your instruction accounts match the program’s expectations
