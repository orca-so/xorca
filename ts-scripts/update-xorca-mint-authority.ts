#!/usr/bin/env tsx

import {
  Connection,
  PublicKey,
  Keypair,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { readFileSync } from 'fs';
import { createSetAuthorityInstruction, AuthorityType } from '@solana/spl-token';
import { XORCA_MINT_ADDRESS, XORCA_STAKING_PROGRAM_ID, RPC_URL } from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 1) {
  console.error('Usage: tsx update-xorca-mint-authority.ts <current-authority-keypair-path>');
  console.error('Example: tsx update-xorca-mint-authority.ts keypairs/authority.json');
  process.exit(1);
}

const [currentAuthorityKeypairPath] = args;

console.log('🚀 Starting xORCA mint authority update...');
console.log(`Current authority keypair: ${currentAuthorityKeypairPath}`);
console.log(`xORCA mint address: ${XORCA_MINT_ADDRESS.toString()}`);
console.log(`xORCA staking program ID: ${XORCA_STAKING_PROGRAM_ID.toString()}`);

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Load current authority keypair
    const currentAuthorityKeypairBytes = JSON.parse(
      readFileSync(currentAuthorityKeypairPath, 'utf8')
    );
    const currentAuthorityKeypair = Keypair.fromSecretKey(
      new Uint8Array(currentAuthorityKeypairBytes)
    );

    console.log(`Current authority public key: ${currentAuthorityKeypair.publicKey.toString()}`);

    // Derive state account address using the program ID
    console.log('🔍 Deriving state account address...');
    const [stateAccount, stateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from('state')],
      XORCA_STAKING_PROGRAM_ID
    );
    console.log(`State account address: ${stateAccount.toString()}`);
    console.log(`State bump: ${stateBump}`);
    console.log(`New mint authority (state account): ${stateAccount.toString()}`);

    // Check current mint authority
    console.log('🔍 Checking current mint authority...');
    const mintInfo = await connection.getParsedAccountInfo(XORCA_MINT_ADDRESS);
    if (mintInfo.value?.data && 'parsed' in mintInfo.value.data) {
      const mintData = mintInfo.value.data.parsed.info;
      console.log(`Current mint authority: ${mintData.mintAuthority || 'None'}`);
      console.log(`Current freeze authority: ${mintData.freezeAuthority || 'None'}`);
      console.log(`Current supply: ${mintData.supply}`);
      console.log(`Decimals: ${mintData.decimals}`);
    }

    // Create the set authority instruction
    console.log('🏗️ Creating set authority instruction...');
    const setAuthorityInstruction = createSetAuthorityInstruction(
      XORCA_MINT_ADDRESS, // mint account
      currentAuthorityKeypair.publicKey, // current authority
      AuthorityType.MintTokens, // authority type
      stateAccount // new authority (state account)
    );

    // Create and send transaction
    console.log('📝 Creating transaction...');
    const transaction = new Transaction().add(setAuthorityInstruction);

    // Get recent blockhash
    console.log('🔍 Getting recent blockhash...');
    const { blockhash } = await connection.getLatestBlockhash('confirmed');
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = currentAuthorityKeypair.publicKey;

    // Sign and send transaction
    console.log('✍️ Signing and sending transaction...');
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [currentAuthorityKeypair],
      {
        commitment: 'confirmed',
        skipPreflight: false,
        preflightCommitment: 'confirmed',
      }
    );

    console.log('✅ Transaction successful!');
    console.log(`Transaction signature: ${signature}`);
    console.log(`Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

    // Verify the authority was updated
    console.log('🔍 Verifying authority update...');
    const updatedMintInfo = await connection.getParsedAccountInfo(XORCA_MINT_ADDRESS);
    if (updatedMintInfo.value?.data && 'parsed' in updatedMintInfo.value.data) {
      const updatedMintData = updatedMintInfo.value.data.parsed.info;
      console.log(`New mint authority: ${updatedMintData.mintAuthority || 'None'}`);

      if (updatedMintData.mintAuthority === stateAccount.toString()) {
        console.log('🎉 Mint authority successfully updated to state account!');
      } else {
        console.log('❌ Mint authority update failed!');
        console.log(`Expected: ${stateAccount.toString()}`);
        console.log(`Actual: ${updatedMintData.mintAuthority || 'None'}`);
      }
    }
  } catch (error) {
    console.error('❌ Error updating mint authority:', error);
    process.exit(1);
  }
}

main();
