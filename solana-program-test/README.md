# xORCA Program Test Suite

This project contains comprehensive integration tests for the xORCA staking program. The test suite uses `LiteSVM` to mimic real instruction calls while retaining the flexibility of writing arbitrary account data for testing edge cases and complex scenarios.

## Test Coverage

The test suite covers all aspects of the xORCA staking program:

- **Core Functionality Tests**
  - `initialize.rs` - Program initialization and setup
  - `stake.rs` - ORCA staking operations
  - `unstake.rs` - xORCA unstaking operations
  - `withdraw.rs` - Pending withdrawal completion
  - `set.rs` - Program parameter updates

- **Edge Case Tests**
  - `bump_edge_cases.rs` - PDA bump edge cases
  - `dos_protection.rs` - Denial of service protection
  - `vault_inflation.rs` - Vault inflation scenarios
  - `yield_operations.rs` - Yield calculation edge cases

## Test Utilities

The test suite includes comprehensive utilities:

- **Assertions** - Custom assertion helpers for program results
- **Fixtures** - Test data and account setup utilities
- **Flows** - End-to-end test scenarios
- **Types** - Test-specific type definitions

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test initialize

# Run with verbose output
cargo test -- --nocapture
```

## Test Architecture

The tests use LiteSVM to simulate the Solana runtime environment, allowing for:

- Precise control over account states
- Testing of edge cases and error conditions
- Validation of program invariants
- Performance testing of critical paths
