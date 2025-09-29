#!/usr/bin/env tsx

import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { readFileSync } from 'fs';
import { TransactionInstruction } from '@solana/web3.js';
import { XORCA_STAKING_PROGRAM_ID, RPC_URL, STATE_SEED } from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 3) {
  console.error('Usage: tsx set.ts <payer-keypair-path> <operation> <value>');
  console.error('Operations:');
  console.error('  update-cooldown <seconds>     - Update the cooldown period in seconds');
  console.error('  update-authority <address>    - Update the update authority address');
  console.error('');
  console.error('Examples:');
  console.error('  tsx set.ts keypairs/deployer.json update-cooldown 3600');
  console.error(
    '  tsx set.ts keypairs/deployer.json update-authority BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq'
  );
  process.exit(1);
}

const [payerKeypairPath, operation, value] = args;

console.log('🚀 Starting set instruction transaction...');
console.log(`Payer keypair: ${payerKeypairPath}`);
console.log(`Operation: ${operation}`);
console.log(`Value: ${value}`);
console.log(`xORCA staking program ID: ${XORCA_STAKING_PROGRAM_ID.toString()}`);

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Load payer keypair
    const payerKeypairBytes = JSON.parse(readFileSync(payerKeypairPath, 'utf8'));
    const payerKeypair = Keypair.fromSecretKey(new Uint8Array(payerKeypairBytes));

    console.log(`Payer public key: ${payerKeypair.publicKey.toString()}`);

    // Derive state account address
    console.log('🔍 Deriving state account address...');
    const [stateAccount, stateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from(STATE_SEED)],
      XORCA_STAKING_PROGRAM_ID
    );
    console.log(`State account: ${stateAccount.toString()}`);
    console.log(`State bump: ${stateBump}`);

    // Check if state account exists
    console.log('🔍 Checking state account...');
    const stateAccountInfo = await connection.getAccountInfo(stateAccount);
    if (!stateAccountInfo) {
      console.error('❌ State account does not exist! Please initialize the program first.');
      process.exit(1);
    }
    console.log('✅ State account exists');

    // Create instruction data based on operation
    console.log('🏗️ Creating set instruction...');
    let instructionData: Buffer;

    if (operation === 'update-cooldown') {
      const cooldownSeconds = parseInt(value);
      if (isNaN(cooldownSeconds) || cooldownSeconds < 0) {
        console.error('❌ Invalid cooldown period! Must be a non-negative integer.');
        process.exit(1);
      }

      // Create instruction data for UpdateCoolDownPeriod
      // Discriminator for Set instruction: 4 (based on the enum order: Stake=0, Unstake=1, Withdraw=2, Initialize=3, Set=4)
      // Discriminator for UpdateCoolDownPeriod: 0 (first variant of StateUpdateInstruction)
      instructionData = Buffer.alloc(10); // 1 byte for Set discriminator + 1 byte for StateUpdateInstruction discriminator + 8 bytes for i64
      instructionData.writeUInt8(4, 0); // Set instruction discriminator
      instructionData.writeUInt8(0, 1); // UpdateCoolDownPeriod discriminator
      instructionData.writeBigInt64LE(BigInt(cooldownSeconds), 2); // new_cool_down_period_s as little-endian i64

      console.log(`Setting cooldown period to: ${cooldownSeconds} seconds`);
    } else if (operation === 'update-authority') {
      let newAuthority: PublicKey;
      try {
        newAuthority = new PublicKey(value);
      } catch (error) {
        console.error('❌ Invalid authority address! Must be a valid Solana public key.');
        process.exit(1);
      }

      // Create instruction data for UpdateUpdateAuthority
      // Discriminator for Set instruction: 4 (based on the enum order: Stake=0, Unstake=1, Withdraw=2, Initialize=3, Set=4)
      // Discriminator for UpdateUpdateAuthority: 1 (second variant of StateUpdateInstruction)
      instructionData = Buffer.alloc(34); // 1 byte for Set discriminator + 1 byte for StateUpdateInstruction discriminator + 32 bytes for Pubkey
      instructionData.writeUInt8(4, 0); // Set instruction discriminator
      instructionData.writeUInt8(1, 1); // UpdateUpdateAuthority discriminator
      instructionData.set(newAuthority.toBytes(), 2); // new_authority as 32-byte public key

      console.log(`Setting new authority to: ${newAuthority.toString()}`);
    } else {
      console.error(
        '❌ Invalid operation! Must be either "update-cooldown" or "update-authority".'
      );
      process.exit(1);
    }

    const setInstruction = new TransactionInstruction({
      keys: [
        { pubkey: payerKeypair.publicKey, isSigner: true, isWritable: true }, // update_authority_account
        { pubkey: stateAccount, isSigner: false, isWritable: true }, // state_account
      ],
      programId: XORCA_STAKING_PROGRAM_ID,
      data: instructionData,
    });

    // Create transaction
    console.log('📝 Creating transaction...');
    const transaction = new Transaction();
    transaction.add(setInstruction);

    // Get recent blockhash
    console.log('🔍 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash('confirmed');
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = payerKeypair.publicKey;

    // Sign and send transaction
    console.log('✍️ Signing and sending transaction...');
    const signature = await sendAndConfirmTransaction(connection, transaction, [payerKeypair], {
      commitment: 'confirmed',
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });

    console.log('✅ Set instruction transaction successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    // Display what was updated
    if (operation === 'update-cooldown') {
      console.log(`✅ Cooldown period updated to: ${value} seconds`);
    } else if (operation === 'update-authority') {
      console.log(`✅ Update authority updated to: ${value}`);
    }
  } catch (error) {
    console.error('❌ Error executing set instruction:', error);
    process.exit(1);
  }
}

main();
