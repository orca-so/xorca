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

export const RPC_URL = 'https://api.devnet.solana.com';
export const COMMITMENT = 'confirmed' as const;

// ============================================================================
// PROGRAM IDENTIFIERS
// ============================================================================

export const XORCA_STAKING_PROGRAM_ID = new PublicKey(
  '8joqMXgaBjc2gGtPVGdZ2tBMxzRJ8igw2SCZQAPky5CE'
);

// ============================================================================
// MINT ADDRESSES
// ============================================================================

export const ORCA_MINT_ADDRESS = new PublicKey('51ipJjMd3aSxyy97du4MDU61GQaUCgehVmyHjfojJpxH');
export const XORCA_MINT_ADDRESS = new PublicKey('Cz1vQJVwpD1Gzy4PEw6yxKNq7MxbPA8Ac7wBrieUmdGz');

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

export const DEPLOYER_ADDRESS = new PublicKey('BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq');

// ============================================================================
// PROGRAM SEEDS
// ============================================================================

export const STATE_SEED = 'state';
export const PENDING_WITHDRAW_SEED = 'pending_withdraw';
