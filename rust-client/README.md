# xORCA Rust Client

A Rust client library for interacting with the xORCA staking program on Solana. Provides type-safe interactions with the liquid staking functionality, allowing users to stake ORCA tokens and receive xORCA tokens in return.

## Features

- **Type-safe interactions** with the xORCA staking program
- **Auto-generated code** from the program IDL using Codama
- **WASM support** for use in web applications
- **PDA (Program Derived Address) utilities** for account derivation
- **Math utilities** with WASM compilation support
- **Serialization support** with optional serde integration
- **Complete program coverage** - all instructions, accounts, and errors

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
use solana_program::instruction::Instruction;

// Get the program ID
let program_id = XORCA_STAKING_PROGRAM_ID;

// Create instruction data for staking
let stake_ix = stake::instruction::Stake {
    orca_stake_amount: 1000000, // 1 ORCA token (6 decimals)
};

// Build the instruction
let instruction = stake_ix.instruction();
```

### PDA Derivation

```rust
use xorca::pda::*;

// Derive state PDA
let (state_pda, _bump) = find_state_pda(&program_id);

// Derive pending withdraw PDA
let (pending_withdraw_pda, _bump) = find_pending_withdraw_pda(
    &program_id,
    &unstaker_pubkey,
    0, // withdraw_index
);
```

### Account Data Deserialization

```rust
use xorca::accounts::*;

// Deserialize state account data
let state: State = borsh::from_slice(&account_data)?;

// Access account fields
println!("Cool down period: {}", state.cool_down_period_s);
println!("Escrowed ORCA amount: {}", state.escrowed_orca_amount);
println!("Update authority: {}", state.update_authority);

// Deserialize pending withdraw account data
let pending_withdraw: PendingWithdraw = borsh::from_slice(&pending_withdraw_data)?;

println!("Withdrawable ORCA amount: {}", pending_withdraw.withdrawable_orca_amount);
println!("Withdrawable timestamp: {}", pending_withdraw.withdrawable_timestamp);
```

### WASM Usage (with `wasm` feature)

```rust
use xorca::math::*;

// Math functions available in WASM for conversion calculations
let result = calculate_xorca_amount(orca_amount, total_orca, total_xorca);
```

## Available Instructions

The client provides type-safe wrappers for all program instructions:

- `initialize` - Initialize the staking program
- `stake` - Stake ORCA tokens to receive xORCA
- `unstake` - Unstake xORCA tokens (creates pending withdrawal)
- `withdraw` - Withdraw ORCA from pending withdrawal after cooldown
- `set` - Update program parameters (cooldown period, authority)

## Account Types

The client includes all account types from the program:

- `State` - Main program state account (PDA)
- `PendingWithdraw` - Pending withdrawal accounts for unstaking

## Error Handling

All program errors are available as Rust enums:

```rust
use xorca::errors::*;

match result {
    Ok(_) => println!("Success!"),
    Err(XorcaStakingProgramError::InsufficientFunds) => {
        println!("Insufficient funds for operation");
    }
    Err(XorcaStakingProgramError::CooldownNotElapsed) => {
        println!("Cooldown period has not elapsed");
    }
    Err(XorcaStakingProgramError::InvalidAuthority) => {
        println!("Invalid authority provided");
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
