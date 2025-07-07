# xORCA Rust Client

A Rust client library for interacting with the xORCA staking program on Solana.

## Features

- **Type-safe interactions** with the xORCA staking program
- **Auto-generated code** from the program IDL using Codama
- **WASM support** for use in web applications
- **PDA (Program Derived Address) utilities** for account derivation
- **Math utilities** with WASM compilation support
- **Serialization support** with optional serde integration

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
xorca = { path = "path/to/xorca/rust-client" }
```

## Features

The crate supports several optional features:

- `serde` - Enable serde serialization/deserialization
- `fetch` - Enable Solana client integration for fetching account data
- `floats` - Enable floating-point math operations (default)
- `wasm` - Enable WASM compilation for web use

### Example with features:

```toml
[dependencies]
xorca = { path = "path/to/xorca/rust-client", features = ["serde", "fetch", "wasm"] }
```

## Usage

### Basic Program Interaction

```rust
use xorca::*;

// Get the program ID
let program_id = XORCA_STAKING_PROGRAM_ID;

// Create instruction data for staking
let stake_ix = stake::instruction::Stake {
    amount: 1000000, // 1 ORCA token (assuming 6 decimals)
    // ... other fields
};

// Build the instruction
let instruction = stake_ix.instruction();
```

### PDA Derivation

```rust
use xorca::pda::*;

// Derive staking pool PDA
let (staking_pool_pda, _bump) = find_staking_pool_pda(&stake_token_mint);

// Derive pending withdraw PDA
let (pending_withdraw_pda, _bump) = find_pending_withdraw_pda(
    &staking_pool_pda,
    &user_pubkey,
    0,
);
```

### Account Data Deserialization

```rust
use xorca::accounts::*;

// Deserialize staking pool account data
let staking_pool: StakingPool = borsh::from_slice(&account_data)?;

// Access account fields
println!("Total staked: {}", staking_pool.total_staked);
println!("Reward rate: {}", staking_pool.reward_rate);
```

### WASM Usage (with `wasm` feature)

```rust
use xorca::math::*;

// Math functions available in WASM
let result = calculate_rewards(amount, rate, duration);
```

## Available Instructions

The client provides type-safe wrappers for all program instructions:

- `initialize` - Initialize a new staking pool
- `stake` - Stake ORCA tokens
- `unstake` - Unstake ORCA tokens
- `claim` - Claim staking rewards
- `withdraw` - Withdraw pending claims
- `cancel_stake` - Cancel a pending stake
- `set` - Update pool parameters

## Account Types

The client includes all account types from the program:

- `StakingPool` - Main staking pool account
- `PendingClaim` - Pending reward claims
- `PendingWithdraw` - Pending withdrawals

## Error Handling

All program errors are available as Rust enums:

```rust
use xorca::errors::*;

match result {
    Ok(_) => println!("Success!"),
    Err(XorcaStakingProgramError::InsufficientFunds) => {
        println!("Insufficient funds for operation");
    }
    Err(e) => println!("Other error: {:?}", e),
}
```

## Development

### Building

```bash
# Build with default features
cargo build

# Build with WASM support
cargo build --features wasm

# Build with all features
cargo build --features "serde,fetch,wasm"
```

### Testing

```bash
cargo test
```

## Code Generation

This client is auto-generated from the Solana program IDL using the [Codama](https://github.com/codama-ai/codama) framework. The generated code includes:

- Type-safe instruction builders
- Account data structures
- Error types
- Program constants

To regenerate the code after program changes:

```bash
yarn generate
```

## License

See [LICENSE](../LICENSE) for details.
