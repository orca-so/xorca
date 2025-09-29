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
  createTransferInstruction,
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { ORCA_MINT_ADDRESS, RPC_URL } from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 3) {
  console.error(
    'Usage: tsx transfer-orca.ts <sender-keypair-path> <recipient-publickey> <orca-amount>'
  );
  console.error('Example: tsx transfer-orca.ts keypairs/sender.json <recipient-publickey> 1000000');
  console.error('Note: ORCA amount should be in smallest units (6 decimals)');
  process.exit(1);
}

const [senderKeypairPath, recipientPublicKeyStr, orcaAmountStr] = args;
const orcaAmount = BigInt(orcaAmountStr);

console.log('🚀 Starting ORCA transfer...');
console.log(`Sender keypair: ${senderKeypairPath}`);
console.log(`Recipient: ${recipientPublicKeyStr}`);
console.log(`ORCA amount: ${orcaAmount.toString()}`);
console.log(`ORCA mint address: ${ORCA_MINT_ADDRESS.toString()}`);

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Load sender keypair
    const senderKeypairBytes = JSON.parse(readFileSync(senderKeypairPath, 'utf8'));
    const senderKeypair = Keypair.fromSecretKey(new Uint8Array(senderKeypairBytes));

    console.log(`Sender public key: ${senderKeypair.publicKey.toString()}`);

    // Parse recipient public key
    const recipientPublicKey = new PublicKey(recipientPublicKeyStr);
    console.log(`Recipient public key: ${recipientPublicKey.toString()}`);

    // Derive sender's ORCA ATA
    console.log('🔍 Deriving sender ORCA ATA...');
    const senderOrcaAta = getAssociatedTokenAddressSync(ORCA_MINT_ADDRESS, senderKeypair.publicKey);
    console.log(`Sender ORCA ATA: ${senderOrcaAta.toString()}`);

    // Derive recipient's ORCA ATA
    console.log('🔍 Deriving recipient ORCA ATA...');
    const recipientOrcaAta = getAssociatedTokenAddressSync(ORCA_MINT_ADDRESS, recipientPublicKey);
    console.log(`Recipient ORCA ATA: ${recipientOrcaAta.toString()}`);

    // Check sender's ORCA balance
    console.log('🔍 Checking sender ORCA balance...');
    const senderBalance = await connection.getTokenAccountBalance(senderOrcaAta);
    console.log(`Sender ORCA balance: ${senderBalance.value.amount}`);

    if (BigInt(senderBalance.value.amount) < orcaAmount) {
      console.error(`❌ Insufficient ORCA balance!`);
      console.error(`Required: ${orcaAmount.toString()}`);
      console.error(`Available: ${senderBalance.value.amount}`);
      process.exit(1);
    }

    // Check if recipient ORCA ATA exists, create if necessary
    console.log('🔍 Checking recipient ORCA ATA...');
    const recipientAtaInfo = await connection.getAccountInfo(recipientOrcaAta);
    let needsAtaCreation = false;

    if (!recipientAtaInfo) {
      console.log('📝 Recipient ORCA ATA does not exist, will create it');
      needsAtaCreation = true;
    } else {
      console.log('✅ Recipient ORCA ATA exists');
    }

    // Check recipient's current ORCA balance (only if ATA exists)
    let recipientBalance = { value: { amount: '0' } };
    if (!needsAtaCreation) {
      console.log('🔍 Checking recipient ORCA balance...');
      recipientBalance = await connection.getTokenAccountBalance(recipientOrcaAta);
      console.log(`Recipient ORCA balance: ${recipientBalance.value.amount}`);
    } else {
      console.log('📝 Recipient ORCA ATA will be created, starting balance is 0');
    }

    // Create the transfer instruction
    console.log('🏗️ Creating transfer instruction...');
    const transferInstruction = createTransferInstruction(
      senderOrcaAta, // source
      recipientOrcaAta, // destination
      senderKeypair.publicKey, // authority
      orcaAmount, // amount
      [], // multiSigners
      TOKEN_PROGRAM_ID // programId
    );

    // Create transaction with ATA creation if needed
    console.log('📝 Creating transaction...');
    const transaction = new Transaction();

    // Add ATA creation instruction if needed
    if (needsAtaCreation) {
      console.log('🏗️ Adding ATA creation instruction...');
      const createAtaInstruction = createAssociatedTokenAccountInstruction(
        senderKeypair.publicKey, // payer
        recipientOrcaAta, // ata
        recipientPublicKey, // owner
        ORCA_MINT_ADDRESS, // mint
        TOKEN_PROGRAM_ID, // tokenProgram
        ASSOCIATED_TOKEN_PROGRAM_ID // ataProgram
      );
      transaction.add(createAtaInstruction);
    }

    // Add transfer instruction
    transaction.add(transferInstruction);

    // Get recent blockhash
    console.log('🔍 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash('confirmed');
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = senderKeypair.publicKey;

    // Sign and send transaction
    console.log('✍️ Signing and sending transaction...');
    const signature = await sendAndConfirmTransaction(connection, transaction, [senderKeypair], {
      commitment: 'confirmed',
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });

    console.log('✅ ORCA transfer successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    // Check final balances
    console.log('🔍 Checking final balances...');
    const finalSenderBalance = await connection.getTokenAccountBalance(senderOrcaAta);
    const finalRecipientBalance = await connection.getTokenAccountBalance(recipientOrcaAta);

    console.log(`Final sender ORCA balance: ${finalSenderBalance.value.amount}`);
    console.log(`Final recipient ORCA balance: ${finalRecipientBalance.value.amount}`);

    const orcaTransferred =
      BigInt(senderBalance.value.amount) - BigInt(finalSenderBalance.value.amount);
    const orcaReceived =
      BigInt(finalRecipientBalance.value.amount) - BigInt(recipientBalance.value.amount);

    console.log(`ORCA transferred: ${orcaTransferred.toString()}`);
    console.log(`ORCA received by recipient: ${orcaReceived.toString()}`);

    if (orcaTransferred === orcaAmount && orcaReceived === orcaAmount) {
      console.log('🎉 Transfer completed successfully!');
    } else {
      console.log('⚠️ Transfer amounts do not match expected values');
    }
  } catch (error) {
    console.error('❌ Error transferring ORCA:', error);
    process.exit(1);
  }
}

main();
