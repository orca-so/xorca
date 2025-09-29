#!/usr/bin/env tsx

import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { readFileSync } from 'fs';
import {
  XORCA_STAKING_PROGRAM_ID,
  ORCA_MINT_ADDRESS,
  XORCA_MINT_ADDRESS,
  DEPLOYER_ADDRESS,
  RPC_URL,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
} from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 3) {
  console.error(
    'Usage: tsx initialize.ts <deployer-keypair-path> <update-authority-keypair-path> <cool-down-period-seconds>'
  );
  console.error('Example: tsx initialize.ts keypairs/deployer.json keypairs/authority.json 3600');
  process.exit(1);
}

const [deployerKeypairPath, updateAuthorityKeypairPath, coolDownPeriodStr] = args;
const coolDownPeriod = parseInt(coolDownPeriodStr);

console.log('🚀 Starting initialize transaction...');
console.log(`Deployer keypair: ${deployerKeypairPath}`);
console.log(`Update authority keypair: ${updateAuthorityKeypairPath}`);
console.log(`Cool down period: ${coolDownPeriod} seconds`);

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Load keypairs
    const deployerKeypairBytes = JSON.parse(readFileSync(deployerKeypairPath, 'utf8'));
    const deployerKeypair = Keypair.fromSecretKey(new Uint8Array(deployerKeypairBytes));

    const updateAuthorityKeypairBytes = JSON.parse(
      readFileSync(updateAuthorityKeypairPath, 'utf8')
    );
    const updateAuthorityKeypair = Keypair.fromSecretKey(
      new Uint8Array(updateAuthorityKeypairBytes)
    );

    const deployerAddress = deployerKeypair.publicKey;
    const updateAuthorityAddress = updateAuthorityKeypair.publicKey;

    console.log(`Deployer address: ${deployerAddress}`);
    console.log(`Update authority address: ${updateAuthorityAddress}`);

    // Verify deployer address matches the expected deployer
    if (!deployerAddress.equals(DEPLOYER_ADDRESS)) {
      console.error(
        `❌ Deployer address ${deployerAddress} does not match expected deployer ${DEPLOYER_ADDRESS}`
      );
      process.exit(1);
    }

    // Derive required addresses
    const [stateAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from('state')],
      XORCA_STAKING_PROGRAM_ID
    );

    const [vaultAddress] = PublicKey.findProgramAddressSync(
      [stateAddress.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), ORCA_MINT_ADDRESS.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    console.log('📋 Account addresses:');
    console.log(`State: ${stateAddress}`);
    console.log(`Vault: ${vaultAddress}`);
    console.log(`xORCA Mint: ${XORCA_MINT_ADDRESS}`);
    console.log(`ORCA Mint: ${ORCA_MINT_ADDRESS}`);

    // Create initialize instruction data
    const instructionData = Buffer.concat([
      Buffer.from([3]), // discriminator for initialize
      Buffer.alloc(8), // cool_down_period_s (i64) - will be filled below
    ]);

    // Write the cool down period as i64 (little-endian)
    const coolDownBuffer = Buffer.alloc(8);
    coolDownBuffer.writeBigInt64LE(BigInt(coolDownPeriod), 0);
    instructionData.set(coolDownBuffer, 1);

    // Create the initialize instruction
    const initializeInstruction = {
      programId: XORCA_STAKING_PROGRAM_ID,
      keys: [
        { pubkey: deployerAddress, isSigner: true, isWritable: true }, // payer_account
        { pubkey: updateAuthorityAddress, isSigner: true, isWritable: true }, // update_authority_account
        { pubkey: stateAddress, isSigner: false, isWritable: true }, // state_account
        { pubkey: vaultAddress, isSigner: false, isWritable: true }, // vault_account
        { pubkey: XORCA_MINT_ADDRESS, isSigner: false, isWritable: false }, // xorca_mint_account
        { pubkey: ORCA_MINT_ADDRESS, isSigner: false, isWritable: false }, // orca_mint_account
        { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false }, // system_program_account
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // token_program_account
        { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // associated_token_program_account
      ],
      data: instructionData,
    };

    console.log('📝 Initialize instruction created');

    // Create and send transaction
    const transaction = new Transaction().add(initializeInstruction);

    console.log('✍️ Sending transaction...');
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [deployerKeypair, updateAuthorityKeypair],
      { commitment: 'confirmed' }
    );

    console.log('✅ Initialize transaction successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    console.log('\n📊 Program initialized with:');
    console.log(`- State account: ${stateAddress}`);
    console.log(`- Vault account: ${vaultAddress}`);
    console.log(`- Update authority: ${updateAuthorityAddress}`);
    console.log(`- Cool down period: ${coolDownPeriod} seconds`);
  } catch (error) {
    console.error('❌ Error during initialize transaction:');
    console.error(error);
    process.exit(1);
  }
}

main().catch(console.error);
