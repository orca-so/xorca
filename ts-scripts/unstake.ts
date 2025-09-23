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
if (args.length < 3) {
  console.error('Usage: tsx unstake.ts <staker-keypair-path> <xorca-amount> <withdraw-index>');
  console.error('Example: tsx unstake.ts keypairs/staker.json 1000000 0');
  process.exit(1);
}

const [stakerKeypairPath, xorcaAmountStr, withdrawIndexStr] = args;
const xorcaAmount = BigInt(xorcaAmountStr);
const withdrawIndex = parseInt(withdrawIndexStr, 10);

console.log('🚀 Starting unstake transaction...');
console.log(`Staker keypair: ${stakerKeypairPath}`);
console.log(`xORCA amount to unstake: ${xorcaAmount}`);
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

    // Staker's xORCA ATA
    const stakerXorcaAta = getAssociatedTokenAddressSync(
      XORCA_MINT_ADDRESS,
      stakerKeypair.publicKey,
      false, // allowOwnerOffCurve
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log(`Staker's xORCA ATA: ${stakerXorcaAta.toString()}`);

    // Check staker's xORCA balance
    console.log('🔍 Checking staker xORCA balance...');
    const stakerXorcaAccountInfo = await connection.getParsedAccountInfo(stakerXorcaAta);
    if (!stakerXorcaAccountInfo.value) {
      console.error(`❌ Staker's xORCA ATA not found: ${stakerXorcaAta.toString()}`);
      console.error('Please stake some ORCA first to receive xORCA tokens.');
      process.exit(1);
    }
    const stakerXorcaBalance = BigInt(
      (stakerXorcaAccountInfo.value.data as any).parsed.info.tokenAmount.amount
    );
    console.log(`Staker's current xORCA balance: ${stakerXorcaBalance}`);

    if (stakerXorcaBalance < xorcaAmount) {
      console.error(
        `❌ Insufficient xORCA funds. Required: ${xorcaAmount}, Available: ${stakerXorcaBalance}`
      );
      process.exit(1);
    }
    console.log('✅ Sufficient xORCA funds for unstaking.');

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

    // Derive pending withdraw account
    console.log('🔍 Deriving pending withdraw account...');
    const [pendingWithdrawAccount] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('pending_withdraw'),
        stakerKeypair.publicKey.toBuffer(),
        Buffer.from([withdrawIndex]),
      ],
      XORCA_STAKING_PROGRAM_ID
    );
    console.log(`Pending withdraw account: ${pendingWithdrawAccount.toString()}`);

    // Create the unstake instruction manually
    console.log('🏗️ Creating unstake instruction...');

    // Create instruction data (discriminator + xorcaUnstakeAmount + withdrawIndex)
    const instructionData = Buffer.alloc(10); // 1 byte discriminator + 8 bytes u64 + 1 byte u8
    instructionData.writeUInt8(1, 0); // discriminator for unstake instruction (1)
    instructionData.writeBigUInt64LE(xorcaAmount, 1); // xorcaUnstakeAmount as little-endian u64
    instructionData.writeUInt8(withdrawIndex, 9); // withdrawIndex as u8

    const unstakeInstruction = new TransactionInstruction({
      keys: [
        { pubkey: stakerKeypair.publicKey, isSigner: true, isWritable: true }, // unstaker_account
        { pubkey: stateAccount, isSigner: false, isWritable: true }, // state_account
        { pubkey: pendingWithdrawAccount, isSigner: false, isWritable: true }, // pending_withdraw_account
        { pubkey: stakerXorcaAta, isSigner: false, isWritable: true }, // unstaker_xorca_ata
        { pubkey: XORCA_MINT_ADDRESS, isSigner: false, isWritable: true }, // xorca_mint_account
        { pubkey: ORCA_MINT_ADDRESS, isSigner: false, isWritable: false }, // orca_mint_account
        { pubkey: vaultAccount, isSigner: false, isWritable: true }, // vault_account
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

    // Add unstake instruction
    transaction.add(unstakeInstruction);

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

    console.log('✅ Unstake transaction successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    // Verify balances after unstaking
    console.log('🔍 Verifying balances after unstaking...');
    const finalStakerXorcaAccountInfo = await connection.getParsedAccountInfo(stakerXorcaAta);
    const finalStakerXorcaBalance = BigInt(
      (finalStakerXorcaAccountInfo.value?.data as any)?.parsed.info.tokenAmount.amount || 0
    );
    console.log(`Staker's final xORCA balance: ${finalStakerXorcaBalance}`);

    const finalStakerOrcaAccountInfo = await connection.getParsedAccountInfo(stakerOrcaAta);
    const finalStakerOrcaBalance = BigInt(
      (finalStakerOrcaAccountInfo.value?.data as any)?.parsed.info.tokenAmount.amount || 0
    );
    console.log(`Staker's final ORCA balance: ${finalStakerOrcaBalance}`);

    const xorcaUnstaked = stakerXorcaBalance - finalStakerXorcaBalance;
    const orcaReceived = finalStakerOrcaBalance - (stakerXorcaBalance > 0 ? BigInt(0) : BigInt(0));
    console.log(`xORCA unstaked: ${xorcaUnstaked}`);
    console.log(`ORCA received: ${orcaReceived}`);
  } catch (error) {
    console.error('❌ Error unstaking xORCA:', error);
    process.exit(1);
  }
}

main();
