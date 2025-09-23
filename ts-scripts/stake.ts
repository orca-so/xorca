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
import { TransactionInstruction } from '@solana/web3.js';
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
  console.error('Usage: tsx stake.ts <staker-keypair-path> <orca-amount>');
  console.error('Example: tsx stake.ts keypairs/staker.json 1000000');
  console.error('Note: ORCA amount should be in smallest units (6 decimals)');
  process.exit(1);
}

const [stakerKeypairPath, orcaAmountStr] = args;
const orcaAmount = BigInt(orcaAmountStr);

console.log('🚀 Starting stake transaction...');
console.log(`Staker keypair: ${stakerKeypairPath}`);
console.log(`ORCA amount to stake: ${orcaAmount.toString()}`);
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

    // Derive state account address
    console.log('🔍 Deriving state account address...');
    const [stateAccount, stateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from('state')],
      XORCA_STAKING_PROGRAM_ID
    );
    console.log(`State account: ${stateAccount.toString()}`);
    console.log(`State bump: ${stateBump}`);

    // Derive vault account address
    console.log('🔍 Deriving vault account address...');
    const [vaultAccount, vaultBump] = PublicKey.findProgramAddressSync(
      [stateAccount.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), ORCA_MINT_ADDRESS.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log(`Vault account: ${vaultAccount.toString()}`);
    console.log(`Vault bump: ${vaultBump}`);

    // Derive staker's ORCA ATA
    console.log('🔍 Deriving staker ORCA ATA...');
    const stakerOrcaAta = getAssociatedTokenAddressSync(ORCA_MINT_ADDRESS, stakerKeypair.publicKey);
    console.log(`Staker ORCA ATA: ${stakerOrcaAta.toString()}`);

    // Derive staker's xORCA ATA
    console.log('🔍 Deriving staker xORCA ATA...');
    const stakerXorcaAta = getAssociatedTokenAddressSync(
      XORCA_MINT_ADDRESS,
      stakerKeypair.publicKey
    );
    console.log(`Staker xORCA ATA: ${stakerXorcaAta.toString()}`);

    // Check staker's ORCA balance
    console.log('🔍 Checking staker ORCA balance...');
    const orcaBalance = await connection.getTokenAccountBalance(stakerOrcaAta);
    console.log(`Staker ORCA balance: ${orcaBalance.value.amount}`);

    if (BigInt(orcaBalance.value.amount) < orcaAmount) {
      console.error(`❌ Insufficient ORCA balance!`);
      console.error(`Required: ${orcaAmount.toString()}`);
      console.error(`Available: ${orcaBalance.value.amount}`);
      process.exit(1);
    }

    // Check if staker has xORCA ATA (create if needed)
    console.log('🔍 Checking staker xORCA ATA...');
    const xorcaAtaInfo = await connection.getAccountInfo(stakerXorcaAta);
    let needsXorcaAtaCreation = false;

    if (!xorcaAtaInfo) {
      console.log('📝 Staker xORCA ATA does not exist, will be created during transaction');
      needsXorcaAtaCreation = true;
    } else {
      console.log('✅ Staker xORCA ATA exists');
    }

    // Create the stake instruction manually
    console.log('🏗️ Creating stake instruction...');

    // Create instruction data (discriminator + orcaStakeAmount)
    const instructionData = Buffer.alloc(9); // 1 byte discriminator + 8 bytes u64
    instructionData.writeUInt8(0, 0); // discriminator for stake instruction
    instructionData.writeBigUInt64LE(orcaAmount, 1); // orcaStakeAmount as little-endian u64

    const stakeInstruction = new TransactionInstruction({
      keys: [
        { pubkey: stakerKeypair.publicKey, isSigner: true, isWritable: true }, // stakerAccount
        { pubkey: vaultAccount, isSigner: false, isWritable: true }, // vaultAccount
        { pubkey: stakerOrcaAta, isSigner: false, isWritable: true }, // stakerOrcaAta
        { pubkey: stakerXorcaAta, isSigner: false, isWritable: true }, // stakerXorcaAta
        { pubkey: XORCA_MINT_ADDRESS, isSigner: false, isWritable: true }, // xorcaMintAccount
        { pubkey: stateAccount, isSigner: false, isWritable: false }, // stateAccount
        { pubkey: ORCA_MINT_ADDRESS, isSigner: false, isWritable: false }, // orcaMintAccount
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // tokenProgramAccount
      ],
      programId: XORCA_STAKING_PROGRAM_ID,
      data: instructionData,
    });

    // Create transaction with ATA creation if needed
    console.log('📝 Creating transaction...');
    const transaction = new Transaction();

    // Add xORCA ATA creation instruction if needed
    if (needsXorcaAtaCreation) {
      console.log('🏗️ Adding xORCA ATA creation instruction...');
      const createXorcaAtaInstruction = createAssociatedTokenAccountInstruction(
        stakerKeypair.publicKey, // payer
        stakerXorcaAta, // ata
        stakerKeypair.publicKey, // owner
        XORCA_MINT_ADDRESS, // mint
        TOKEN_PROGRAM_ID, // tokenProgram
        ASSOCIATED_TOKEN_PROGRAM_ID // ataProgram
      );
      transaction.add(createXorcaAtaInstruction);
    }

    // Add stake instruction
    transaction.add(stakeInstruction);

    // Get recent blockhash
    console.log('🔍 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash('confirmed');
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = stakerKeypair.publicKey;

    // Sign and send transaction
    console.log('✍️ Signing and sending transaction...');
    const signature = await sendAndConfirmTransaction(connection, transaction, [stakerKeypair], {
      commitment: 'confirmed',
      skipPreflight: false,
      preflightCommitment: 'confirmed',
    });

    console.log('✅ Stake transaction successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    // Check final balances
    console.log('🔍 Checking final balances...');
    const finalOrcaBalance = await connection.getTokenAccountBalance(stakerOrcaAta);
    const finalXorcaBalance = await connection.getTokenAccountBalance(stakerXorcaAta);

    console.log(`Final ORCA balance: ${finalOrcaBalance.value.amount}`);
    console.log(`Final xORCA balance: ${finalXorcaBalance.value.amount}`);

    const orcaStaked = BigInt(orcaBalance.value.amount) - BigInt(finalOrcaBalance.value.amount);
    console.log(`ORCA staked: ${orcaStaked.toString()}`);
    console.log(`xORCA received: ${finalXorcaBalance.value.amount}`);
  } catch (error) {
    console.error('❌ Error staking ORCA:', error);
    process.exit(1);
  }
}

main();
