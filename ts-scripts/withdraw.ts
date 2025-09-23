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
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
} from '@solana/spl-token';
import { SystemProgram, TransactionInstruction } from '@solana/web3.js';
import {
  ORCA_MINT_ADDRESS,
  XORCA_MINT_ADDRESS,
  XORCA_STAKING_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  RPC_URL,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 2) {
  console.error('Usage: tsx withdraw.ts <staker-keypair-path> <withdraw-index>');
  console.error('Example: tsx withdraw.ts keypairs/staker.json 0');
  process.exit(1);
}

const [stakerKeypairPath, withdrawIndexStr] = args;
const withdrawIndex = parseInt(withdrawIndexStr, 10);

console.log('🚀 Starting withdraw transaction...');
console.log(`Staker keypair: ${stakerKeypairPath}`);
console.log(`Withdraw index: ${withdrawIndex}`);
console.log(`ORCA mint address: ${ORCA_MINT_ADDRESS.toString()}`);
console.log(`xORCA mint address: ${XORCA_MINT_ADDRESS.toString()}`);
console.log(`xORCA staking program ID: ${XORCA_STAKING_PROGRAM_ID.toString()}`);

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Load staker keypair
    const stakerKeypairBytes = JSON.parse(readFileSync(stakerKeypairPath, 'utf8'));
    const stakerKeypair = Keypair.fromSecretKey(new Uint8Array(stakerKeypairBytes));
    console.log(`Staker public key: ${stakerKeypair.publicKey.toString()}`);

    // Derive PDAs and ATAs
    console.log('🔍 Deriving required account addresses...');

    // State Account PDA
    const [stateAccount] = PublicKey.findProgramAddressSync(
      [Buffer.from('state')],
      XORCA_STAKING_PROGRAM_ID
    );
    console.log(`State account: ${stateAccount.toString()}`);

    // Pending Withdraw Account PDA
    const [pendingWithdrawAccount] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('pending_withdraw'),
        stakerKeypair.publicKey.toBuffer(),
        Buffer.from([withdrawIndex]),
      ],
      XORCA_STAKING_PROGRAM_ID
    );
    console.log(`Pending withdraw account: ${pendingWithdrawAccount.toString()}`);

    // Vault Account ATA (for state + ORCA mint)
    const [vaultAccount] = PublicKey.findProgramAddressSync(
      [stateAccount.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), ORCA_MINT_ADDRESS.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log(`Vault account (ATA for state + ORCA): ${vaultAccount.toString()}`);

    // Staker's ORCA ATA
    const stakerOrcaAta = getAssociatedTokenAddressSync(
      ORCA_MINT_ADDRESS,
      stakerKeypair.publicKey,
      false, // allowOwnerOffCurve
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log(`Staker's ORCA ATA: ${stakerOrcaAta.toString()}`);

    // Check if pending withdraw account exists
    console.log('🔍 Checking pending withdraw account...');
    const pendingWithdrawInfo = await connection.getAccountInfo(pendingWithdrawAccount);
    if (!pendingWithdrawInfo) {
      console.error(`❌ Pending withdraw account not found: ${pendingWithdrawAccount.toString()}`);
      console.error('Please unstake some xORCA first to create a pending withdraw account.');
      process.exit(1);
    }
    console.log('✅ Pending withdraw account exists');

    // Read pending withdraw account data to check cooldown
    console.log('🔍 Checking cooldown period...');
    const pendingWithdrawData = pendingWithdrawInfo.data;
    if (pendingWithdrawData.length < 56) {
      // Minimum length check (48 + 8 for timestamp)
      console.error('❌ Invalid pending withdraw account data');
      process.exit(1);
    }

    // Parse the withdrawable timestamp (8 bytes starting at offset 48)
    const withdrawableTimestampBuffer = pendingWithdrawData.slice(48, 56);
    const withdrawableTimestamp = Number(withdrawableTimestampBuffer.readBigInt64LE());
    const currentTimestamp = Math.floor(Date.now() / 1000);
    let secondsRemaining = withdrawableTimestamp - currentTimestamp;

    console.log(`📅 Withdrawable timestamp: ${withdrawableTimestamp}`);
    console.log(`🕐 Current timestamp: ${currentTimestamp}`);
    console.log(`⏰ Seconds remaining: ${secondsRemaining}`);

    if (secondsRemaining > 0) {
      const hours = Math.floor(secondsRemaining / 3600);
      const minutes = Math.floor((secondsRemaining % 3600) / 60);
      const seconds = secondsRemaining % 60;
      console.log(`⏳ Cooldown period remaining: ${hours}h ${minutes}m ${seconds}s`);
    } else {
      console.log('✅ Cooldown period has elapsed - withdrawal should be possible');
    }

    // Check if staker has ORCA ATA (create if needed)
    console.log('🔍 Checking staker ORCA ATA...');
    const orcaAtaInfo = await connection.getAccountInfo(stakerOrcaAta);
    let needsOrcaAtaCreation = false;

    if (!orcaAtaInfo) {
      console.log('📝 Staker ORCA ATA does not exist, will be created during transaction');
      needsOrcaAtaCreation = true;
    } else {
      console.log('✅ Staker ORCA ATA exists');
    }

    // Create the withdraw instruction manually
    console.log('🏗️ Creating withdraw instruction...');

    // Create instruction data (discriminator + withdrawIndex)
    const instructionData = Buffer.alloc(2); // 1 byte discriminator + 1 byte u8
    instructionData.writeUInt8(2, 0); // discriminator for withdraw instruction (2)
    instructionData.writeUInt8(withdrawIndex, 1); // withdrawIndex as u8

    const withdrawInstruction = new TransactionInstruction({
      keys: [
        { pubkey: stakerKeypair.publicKey, isSigner: true, isWritable: true }, // unstaker_account
        { pubkey: stateAccount, isSigner: false, isWritable: true }, // state_account
        { pubkey: pendingWithdrawAccount, isSigner: false, isWritable: true }, // pending_withdraw_account
        { pubkey: stakerOrcaAta, isSigner: false, isWritable: true }, // unstaker_orca_ata
        { pubkey: vaultAccount, isSigner: false, isWritable: true }, // vault_account
        { pubkey: ORCA_MINT_ADDRESS, isSigner: false, isWritable: false }, // orca_mint_account
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // system_program_account
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // token_program_account
      ],
      programId: XORCA_STAKING_PROGRAM_ID,
      data: instructionData,
    });

    // Create transaction with ATA creation if needed
    console.log('📝 Creating transaction...');
    const transaction = new Transaction();

    // Add ORCA ATA creation instruction if needed
    if (needsOrcaAtaCreation) {
      console.log('🏗️ Adding ORCA ATA creation instruction...');
      const createOrcaAtaInstruction = createAssociatedTokenAccountInstruction(
        stakerKeypair.publicKey, // payer
        stakerOrcaAta, // ata
        stakerKeypair.publicKey, // owner
        ORCA_MINT_ADDRESS, // mint
        TOKEN_PROGRAM_ID, // tokenProgram
        ASSOCIATED_TOKEN_PROGRAM_ID // ataProgram
      );
      transaction.add(createOrcaAtaInstruction);
    }

    // Add withdraw instruction
    transaction.add(withdrawInstruction);

    // Get recent blockhash
    console.log('🔍 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash('confirmed');
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = stakerKeypair.publicKey;

    // Sign and send transaction
    console.log('✍️ Signing and sending transaction...');
    console.log('⚠️  Note: This transaction may fail if the cooldown period has not elapsed');

    try {
      const signature = await sendAndConfirmTransaction(connection, transaction, [stakerKeypair], {
        commitment: 'confirmed',
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      });

      console.log('✅ Withdraw transaction successful!');
      console.log(`Transaction signature: ${signature}`);
      console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

      // Verify balances after withdrawal
      console.log('🔍 Verifying balances after withdrawal...');
      const finalStakerOrcaAccountInfo = await connection.getParsedAccountInfo(stakerOrcaAta);
      const finalStakerOrcaBalance = BigInt(
        (finalStakerOrcaAccountInfo.value?.data as any)?.parsed.info.tokenAmount.amount || 0
      );
      console.log(`Staker's final ORCA balance: ${finalStakerOrcaBalance}`);
    } catch (error) {
      console.log('❌ Withdraw transaction failed (expected due to cooldown period)');
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.log('Error details:', errorMessage);

      // Check if it's a cooldown error
      if (
        errorMessage.includes('0x177b') ||
        errorMessage.includes('0x177B') ||
        errorMessage.includes('CoolDownPeriodStillActive')
      ) {
        console.log('🕐 This is expected - the cooldown period has not elapsed yet');
        console.log('⏰ Please wait for the cooldown period to complete before withdrawing');

        // Show remaining time if we calculated it earlier
        if (secondsRemaining > 0) {
          const hours = Math.floor(secondsRemaining / 3600);
          const minutes = Math.floor((secondsRemaining % 3600) / 60);
          const secs = secondsRemaining % 60;
          console.log(`⏳ Time remaining: ${hours}h ${minutes}m ${secs}s`);
        } else {
          console.log('📅 The cooldown period is typically 1800 seconds (30 minutes)');
        }
      } else {
        console.log('❌ Unexpected error:', error);
      }
    }
  } catch (error) {
    console.error('❌ Error withdrawing from pending withdraw:', error);
    process.exit(1);
  }
}

main();
