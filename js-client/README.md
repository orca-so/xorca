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

```typescript
import { Connection, PublicKey, Transaction } from '@solana/web3.js';
import {
  createStakeInstruction,
  createUnstakeInstruction,
  createWithdrawInstruction,
  findStatePda,
  findPendingWithdrawPda,
} from '@orca-so/xorca';

const connection = new Connection('https://api.devnet.solana.com');
const programId = new PublicKey('StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT');
```

### Staking Operations

```typescript
import { createStakeInstruction } from '@orca-so/xorca';

// Create a stake instruction
const stakeInstruction = createStakeInstruction({
  staker: userPublicKey,
  vault: vaultPda,
  stakerOrcaAta: stakerOrcaAta,
  stakerXorcaAta: stakerXorcaAta,
  xorcaMint: xorcaMint,
  state: statePda,
  orcaMint: orcaMint,
  tokenProgram: TOKEN_PROGRAM_ID,
  orcaStakeAmount: 1000000, // 1 ORCA token (6 decimals)
});

// Add to transaction
transaction.add(stakeInstruction);
```

### Unstaking Operations

```typescript
import { createUnstakeInstruction } from '@orca-so/xorca';

// Create an unstake instruction
const unstakeInstruction = createUnstakeInstruction({
  unstaker: userPublicKey,
  state: statePda,
  vault: vaultPda,
  pendingWithdraw: pendingWithdrawPda,
  unstakerXorcaAta: unstakerXorcaAta,
  xorcaMint: xorcaMint,
  orcaMint: orcaMint,
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  xorcaUnstakeAmount: 500000, // 0.5 xORCA tokens
  withdrawIndex: 0,
});

// Add to transaction
transaction.add(unstakeInstruction);
```

### Withdrawing from Pending Withdrawals

```typescript
import { createWithdrawInstruction } from '@orca-so/xorca';

// Create a withdraw instruction
const withdrawInstruction = createWithdrawInstruction({
  unstaker: userPublicKey,
  state: statePda,
  pendingWithdraw: pendingWithdrawPda,
  unstakerOrcaAta: unstakerOrcaAta,
  vault: vaultPda,
  orcaMint: orcaMint,
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_PROGRAM_ID,
  withdrawIndex: 0,
});

// Add to transaction
transaction.add(withdrawInstruction);
```

### PDA Derivation

```typescript
import { findStatePda, findPendingWithdrawPda } from '@orca-so/xorca';

// Derive state PDA
const [statePda, stateBump] = findStatePda({
  programId,
});

// Derive pending withdraw PDA
const [pendingWithdrawPda, pendingWithdrawBump] = findPendingWithdrawPda({
  programId,
  unstaker: userPublicKey,
  withdrawIndex: 0,
});

// Derive vault ATA (Associated Token Account for state + ORCA mint)
const vaultPda = getAssociatedTokenAddressSync(
  orcaMint,
  statePda,
  true // allowOwnerOffCurve
);
```

### Account Data Deserialization

```typescript
import { State, PendingWithdraw } from '@orca-so/xorca';

// Fetch and deserialize state account
const stateAccount = await connection.getAccountInfo(statePda);
const state = State.fromAccountInfo(stateAccount.data);

// Access account fields
console.log('Cool down period:', state.coolDownPeriodS.toString());
console.log('Escrowed ORCA amount:', state.escrowedOrcaAmount.toString());
console.log('Update authority:', state.updateAuthority.toString());

// Fetch and deserialize pending withdraw account
const pendingWithdrawAccount = await connection.getAccountInfo(pendingWithdrawPda);
const pendingWithdraw = PendingWithdraw.fromAccountInfo(pendingWithdrawAccount.data);

console.log('Withdrawable ORCA amount:', pendingWithdraw.withdrawableOrcaAmount.toString());
console.log('Withdrawable timestamp:', pendingWithdraw.withdrawableTimestamp.toString());
```

## 📋 API Reference

### Available Instructions

The client provides type-safe builders for all program instructions:

- `createInitializeInstruction` - Initialize the staking program
- `createStakeInstruction` - Stake ORCA tokens to receive xORCA
- `createUnstakeInstruction` - Unstake xORCA tokens (creates pending withdrawal)
- `createWithdrawInstruction` - Withdraw ORCA from pending withdrawal after cooldown
- `createSetInstruction` - Update program parameters (cooldown period, authority)

### Account Types

The client includes all account types from the program:

- `State` - Main program state account (PDA)
- `PendingWithdraw` - Pending withdrawal accounts for unstaking

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
      case 'CooldownNotElapsed':
        console.error('Cooldown period has not elapsed');
        break;
      case 'InvalidAuthority':
        console.error('Invalid authority provided');
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
