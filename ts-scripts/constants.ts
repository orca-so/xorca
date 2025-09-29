import { PublicKey } from '@solana/web3.js';

/**
 * xORCA Staking Program Constants
 *
 * This file contains all the constant values used across the xORCA staking scripts.
 * All public keys and configuration values are centralized here for easy maintenance.
 */

// ============================================================================
// NETWORK CONFIGURATION
// ============================================================================

export const RPC_URL = 'https://api.mainnet-beta.solana.com';
export const COMMITMENT = 'confirmed' as const;

// ============================================================================
// PROGRAM IDENTIFIERS
// ============================================================================

export const XORCA_STAKING_PROGRAM_ID = new PublicKey(
  'StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT'
);

// ============================================================================
// MINT ADDRESSES
// ============================================================================

export const ORCA_MINT_ADDRESS = new PublicKey('orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE');
export const XORCA_MINT_ADDRESS = new PublicKey('xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N');

// ============================================================================
// SYSTEM PROGRAM IDENTIFIERS
// ============================================================================

export const SYSTEM_PROGRAM_ID = new PublicKey('11111111111111111111111111111111');
export const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'
);

// ============================================================================
// AUTHORIZATION
// ============================================================================

export const DEPLOYER_ADDRESS = new PublicKey('94kZD71sbTKhqhcvY9D9Ra5BsLzKRZgznbBbQpBWmKrT');

// ============================================================================
// PROGRAM SEEDS
// ============================================================================

export const STATE_SEED = 'state';
export const PENDING_WITHDRAW_SEED = 'pending_withdraw';
