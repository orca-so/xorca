# xORCA JavaScript Client

A TypeScript/JavaScript client library for interacting with the xORCA staking program on Solana.

## Features

- **Type-safe interactions** with the xORCA staking program
- **Auto-generated code** from the program IDL using Codama
- **WASM integration** for high-performance math operations
- **Browser and Node.js support** with dual build outputs
- **Solana Kit integration** for seamless Solana development
- **Comprehensive TypeScript types** for all program interactions

## Installation

```bash
npm install @orca-so/xorca
# or
yarn add @orca-so/xorca
```

## Prerequisites

This library requires `@solana/kit` as a peer dependency:

```bash
npm install @solana/kit
# or
yarn add @solana/kit
```

## Usage

### Basic Setup

```typescript
import { Connection, PublicKey } from "@solana/web3.js";
import {
  XORCA_STAKING_PROGRAM_ID,
  createStakeInstruction,
  createUnstakeInstruction,
  findStakingPoolPda,
  findPendingClaimPda,
} from "@orca-so/xorca";

// Connect to Solana
const connection = new Connection("https://api.mainnet-beta.solana.com");

// Program ID
const programId = XORCA_STAKING_PROGRAM_ID;
```

### Staking Operations

```typescript
import { createStakeInstruction } from "@orca-so/xorca";

// Create a stake instruction
const stakeInstruction = createStakeInstruction({
  stakingPool: stakingPoolPda,
  user: userPublicKey,
  userTokenAccount: userTokenAccount,
  amount: 1000000, // 1 ORCA token (assuming 6 decimals)
  // ... other required accounts
});

// Add to transaction
transaction.add(stakeInstruction);
```

### Unstaking Operations

```typescript
import { createUnstakeInstruction } from "@orca-so/xorca";

// Create an unstake instruction
const unstakeInstruction = createUnstakeInstruction({
  stakingPool: stakingPoolPda,
  user: userPublicKey,
  userTokenAccount: userTokenAccount,
  amount: 500000, // 0.5 ORCA tokens
  // ... other required accounts
});

// Add to transaction
transaction.add(unstakeInstruction);
```

### Claiming Rewards

```typescript
import { createClaimInstruction } from "@orca-so/xorca";

// Create a claim instruction
const claimInstruction = createClaimInstruction({
  stakingPool: stakingPoolPda,
  pendingClaim: pendingClaimPda,
  user: userPublicKey,
  // ... other required accounts
});

// Add to transaction
transaction.add(claimInstruction);
```

### PDA Derivation

```typescript
import {
  findStakingPoolPda,
  findPendingClaimPda,
  findPendingWithdrawPda,
} from "@orca-so/xorca";

// Derive staking pool PDA
const [stakingPoolPda, stakingPoolBump] = findStakingPoolPda({
  programId,
});

// Derive pending claim PDA
const [pendingClaimPda, pendingClaimBump] = findPendingClaimPda({
  programId,
  user: userPublicKey,
  stakingPool: stakingPoolPda,
});

// Derive pending withdraw PDA
const [pendingWithdrawPda, pendingWithdrawBump] = findPendingWithdrawPda({
  programId,
  user: userPublicKey,
  stakingPool: stakingPoolPda,
});
```

### Account Data Deserialization

```typescript
import { StakingPool, PendingClaim } from "@orca-so/xorca";

// Fetch and deserialize staking pool account
const stakingPoolAccount = await connection.getAccountInfo(stakingPoolPda);
const stakingPool = StakingPool.fromAccountInfo(stakingPoolAccount.data);

// Access account fields
console.log("Total staked:", stakingPool.totalStaked.toString());
console.log("Reward rate:", stakingPool.rewardRate.toString());
```

### WASM Math Functions

```typescript
import { calculateRewards } from "@orca-so/xorca";

// Use WASM-optimized math functions
const rewards = calculateRewards(amount, rate, duration);
```

## Available Instructions

The client provides type-safe builders for all program instructions:

- `createInitializeInstruction` - Initialize a new staking pool
- `createStakeInstruction` - Stake ORCA tokens
- `createUnstakeInstruction` - Unstake ORCA tokens
- `createClaimInstruction` - Claim staking rewards
- `createWithdrawInstruction` - Withdraw pending claims
- `createCancelStakeInstruction` - Cancel a pending stake
- `createSetInstruction` - Update pool parameters

## Account Types

The client includes all account types from the program:

- `StakingPool` - Main staking pool account
- `PendingClaim` - Pending reward claims
- `PendingWithdraw` - Pending withdrawals

## Error Handling

All program errors are available as TypeScript types:

```typescript
import { XorcaStakingProgramError } from "@orca-so/xorca";

try {
  // Execute transaction
  const signature = await connection.sendTransaction(transaction);
} catch (error) {
  if (error instanceof XorcaStakingProgramError) {
    switch (error.code) {
      case "InsufficientFunds":
        console.error("Insufficient funds for operation");
        break;
      case "InvalidAccount":
        console.error("Invalid account provided");
        break;
      default:
        console.error("Program error:", error.message);
    }
  }
}
```

## Build Outputs

The library provides multiple build outputs:

- **Browser**: `dist/index.browser.js` - Optimized for browser environments
- **Node.js**: `dist/index.node.js` - Optimized for Node.js environments
- **TypeScript**: `dist/index.d.ts` - Type definitions

## Development

### Building

```bash
# Build for both browser and Node.js
yarn build

# Build for browser only
yarn build --mode browser

# Build for Node.js only
yarn build --mode node
```

### Testing

```bash
yarn test
```

### Formatting

```bash
yarn fmt
```

## Code Generation

This client is auto-generated from the Solana program IDL using the [Codama](https://github.com/codama-ai/codama) framework. The generated code includes:

- Type-safe instruction builders
- Account data structures and deserializers
- Error types
- Program constants
- WASM bindings for math operations

To regenerate the code after program changes:

```bash
yarn generate
```

## Examples

### Complete Staking Flow

```typescript
import {
  Connection,
  Transaction,
  PublicKey,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  createStakeInstruction,
  findStakingPoolPda,
  XORCA_STAKING_PROGRAM_ID,
} from "@orca-so/xorca";

async function stakeTokens(
  connection: Connection,
  user: Keypair,
  amount: number
) {
  const programId = XORCA_STAKING_PROGRAM_ID;
  const [stakingPoolPda] = findStakingPoolPda({ programId });

  const instruction = createStakeInstruction({
    stakingPool: stakingPoolPda,
    user: user.publicKey,
    userTokenAccount: userTokenAccount,
    amount,
    // ... other required accounts
  });

  const transaction = new Transaction().add(instruction);

  const signature = await sendAndConfirmTransaction(connection, transaction, [
    user,
  ]);

  console.log("Stake transaction:", signature);
}
```

## License

See [LICENSE](../LICENSE) for details.
