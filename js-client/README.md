# @orca-so/xorca

[![npm version](https://badge.fury.io/js/%40orca-so%2Fxorca.svg)](https://badge.fury.io/js/%40orca-so%2Fxorca)
[![License: Custom](https://img.shields.io/badge/License-Custom-blue.svg)](../LICENSE)

A TypeScript/JavaScript client library for interacting with the xORCA staking program on Solana. Built with type safety, performance, and developer experience in mind.

## ✨ Features

- 🔒 **Type-safe interactions** with the xORCA staking program
- ⚡ **WASM-optimized math operations** for high-performance calculations
- 🌐 **Universal compatibility** - works in both browser and Node.js environments
- 🛠️ **Auto-generated code** from the program IDL using Codama
- 📦 **Zero-config setup** with comprehensive TypeScript definitions
- 🔗 **Solana Kit integration** for seamless Solana development
- 🎯 **Complete program coverage** - all instructions, accounts, and errors

## 📦 Installation

```bash
npm install @orca-so/xorca
```

Or with yarn:

```bash
yarn add @orca-so/xorca
```

## 🔧 Prerequisites

This library requires the following peer dependencies:

```bash
npm install @solana/kit @solana-program/token
```

Or with yarn:

```bash
yarn add @solana/kit @solana-program/token
```

## 📖 Usage Guide

### Basic Setup

### Staking Operations

```typescript
import { createStakeInstruction } from '@orca-so/xorca';

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
import { createUnstakeInstruction } from '@orca-so/xorca';

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
import { createClaimInstruction } from '@orca-so/xorca';

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
import { findStakingPoolPda, findPendingClaimPda, findPendingWithdrawPda } from '@orca-so/xorca';

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
import { StakingPool, PendingClaim } from '@orca-so/xorca';

// Fetch and deserialize staking pool account
const stakingPoolAccount = await connection.getAccountInfo(stakingPoolPda);
const stakingPool = StakingPool.fromAccountInfo(stakingPoolAccount.data);

// Access account fields
console.log('Total staked:', stakingPool.totalStaked.toString());
console.log('Reward rate:', stakingPool.rewardRate.toString());
```

## 📋 API Reference

### Available Instructions

The client provides type-safe builders for all program instructions:

- `createInitializeInstruction` - Initialize a new staking pool
- `createStakeInstruction` - Stake ORCA tokens
- `createUnstakeInstruction` - Unstake ORCA tokens
- `createClaimInstruction` - Claim staking rewards
- `createWithdrawInstruction` - Withdraw pending claims
- `createCancelStakeInstruction` - Cancel a pending stake
- `createSetInstruction` - Update pool parameters

### Account Types

The client includes all account types from the program:

- `StakingPool` - Main staking pool account
- `PendingClaim` - Pending reward claims
- `PendingWithdraw` - Pending withdrawals

### Error Handling

All program errors are available as TypeScript types:

```typescript
import { XorcaStakingProgramError } from '@orca-so/xorca';

try {
  // Execute transaction
  const signature = await connection.sendTransaction(transaction);
} catch (error) {
  if (error instanceof XorcaStakingProgramError) {
    switch (error.code) {
      case 'InsufficientFunds':
        console.error('Insufficient funds for operation');
        break;
      case 'InvalidAccount':
        console.error('Invalid account provided');
        break;
      default:
        console.error('Program error:', error.message);
    }
  }
}
```

## 🏗️ Build Outputs

The library provides multiple build outputs for different environments:

- **Browser**: `dist/index.browser.js` - Optimized for browser environments
- **Node.js**: `dist/index.node.js` - Optimized for Node.js environments
- **TypeScript**: `dist/index.d.ts` - Complete type definitions

## 🔧 Development

### Building

```bash
# Build for both browser and Node.js
npm run build

# Build for browser only
npm run build -- --mode browser

# Build for Node.js only
npm run build -- --mode node
```

### Testing

```bash
npm test
```

### Formatting

```bash
npm run fmt
```

## 🔄 Code Generation

This client is auto-generated from the Solana program IDL using the [Codama](https://github.com/codama-ai/codama) framework. The generated code includes:

- Type-safe instruction builders
- Account data structures and deserializers
- Error types
- Program constants
- WASM bindings for math operations

## 📊 Package Info

- **Package Size**: ~60KB (compressed)
- **Bundle Size**: ~122KB (uncompressed)
- **TypeScript Support**: Full type definitions included
- **Node.js Support**: >=18.0.0
- **Browser Support**: Modern browsers with ES2020 support
