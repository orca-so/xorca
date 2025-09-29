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
  createTransferInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { ORCA_MINT_ADDRESS, XORCA_STAKING_PROGRAM_ID, RPC_URL, STATE_SEED } from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 2) {
  console.error('Usage: tsx transfer-orca-to-vault.ts <sender-keypair-path> <orca-amount>');
  console.error('Example: tsx transfer-orca-to-vault.ts keypairs/deployer.json 100000');
  console.error('Note: ORCA amount should be in smallest units (6 decimals)');
  process.exit(1);
}

const [senderKeypairPath, orcaAmountStr] = args;
const orcaAmount = BigInt(parseInt(orcaAmountStr));

console.log('🚀 Starting ORCA transfer to vault...');
console.log(`Sender keypair: ${senderKeypairPath}`);
console.log(`ORCA amount: ${orcaAmount.toString()}`);

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Load sender keypair
    const senderKeypairBytes = JSON.parse(readFileSync(senderKeypairPath, 'utf8'));
    const senderKeypair = Keypair.fromSecretKey(new Uint8Array(senderKeypairBytes));
    const senderAddress = senderKeypair.publicKey;

    console.log(`Sender public key: ${senderAddress.toString()}`);

    // Derive state account address
    const [stateAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from(STATE_SEED)],
      XORCA_STAKING_PROGRAM_ID
    );

    // Derive vault account address (PDA)
    const [vaultAddress] = PublicKey.findProgramAddressSync(
      [stateAddress.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), ORCA_MINT_ADDRESS.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    console.log(`State account: ${stateAddress.toString()}`);
    console.log(`Vault account (PDA): ${vaultAddress.toString()}`);

    // Derive sender's ORCA ATA
    const senderOrcaAta = getAssociatedTokenAddressSync(ORCA_MINT_ADDRESS, senderAddress);

    console.log(`Sender ORCA ATA: ${senderOrcaAta.toString()}`);

    // Check sender ORCA balance
    console.log('🔍 Checking sender ORCA balance...');
    const senderOrcaAccount = await getAccount(connection, senderOrcaAta);
    const senderBalance = senderOrcaAccount.amount;

    console.log(`Sender ORCA balance: ${senderBalance.toString()}`);

    if (senderBalance < orcaAmount) {
      console.error(`❌ Insufficient ORCA balance!`);
      console.error(`Required: ${orcaAmount.toString()}`);
      console.error(`Available: ${senderBalance.toString()}`);
      process.exit(1);
    }

    // Check if vault account exists
    console.log('🔍 Checking vault account...');
    let vaultAccount;
    try {
      vaultAccount = await getAccount(connection, vaultAddress);
      console.log(`Vault account exists with balance: ${vaultAccount.amount.toString()}`);
    } catch (error) {
      console.error('❌ Vault account not found or not accessible');
      console.error('This might mean the program is not initialized or the vault PDA is incorrect');
      process.exit(1);
    }

    // Create transfer instruction
    console.log('🏗️ Creating transfer instruction...');
    const transferInstruction = createTransferInstruction(
      senderOrcaAta, // source
      vaultAddress, // destination (vault PDA)
      senderAddress, // owner of source account
      orcaAmount, // amount
      [], // multiSigners
      TOKEN_PROGRAM_ID // programId
    );

    // Create transaction
    console.log('📝 Creating transaction...');
    const transaction = new Transaction().add(transferInstruction);

    // Get recent blockhash
    console.log('🔍 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = senderAddress;

    // Sign and send transaction
    console.log('✍️ Signing and sending transaction...');
    const signature = await sendAndConfirmTransaction(connection, transaction, [senderKeypair], {
      commitment: 'confirmed',
    });

    console.log('✅ ORCA transfer to vault successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    // Check final balances
    console.log('🔍 Checking final balances...');
    const finalSenderAccount = await getAccount(connection, senderOrcaAta);
    const finalVaultAccount = await getAccount(connection, vaultAddress);

    const finalSenderBalance = finalSenderAccount.amount;
    const finalVaultBalance = finalVaultAccount.amount;

    console.log(`Final sender ORCA balance: ${finalSenderBalance.toString()}`);
    console.log(`Final vault ORCA balance: ${finalVaultBalance.toString()}`);
    console.log(`ORCA transferred: ${orcaAmount.toString()}`);
    console.log(`Vault received: ${finalVaultBalance - (vaultAccount.amount - orcaAmount)} ORCA`);

    console.log('🎉 Transfer to vault completed successfully!');
  } catch (error) {
    console.error('❌ Error transferring ORCA to vault:', error);
    process.exit(1);
  }
}

main().catch(console.error);
