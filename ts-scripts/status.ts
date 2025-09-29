#!/usr/bin/env tsx

import { Connection, PublicKey } from '@solana/web3.js';
import { getAccount, getMint } from '@solana/spl-token';
import {
  ORCA_MINT_ADDRESS,
  XORCA_MINT_ADDRESS,
  XORCA_STAKING_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  RPC_URL,
  STATE_SEED,
} from './constants';

// Virtual amounts for DOS protection (matching the Rust implementation)
const VIRTUAL_XORCA_SUPPLY = 100n;
const VIRTUAL_NON_ESCROWED_ORCA_AMOUNT = 100n;

console.log('🔍 Fetching xORCA Staking Program Status...');
console.log('='.repeat(60));

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Derive state account address
    const [stateAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from(STATE_SEED)],
      XORCA_STAKING_PROGRAM_ID
    );

    // Derive vault account address
    const [vaultAddress] = PublicKey.findProgramAddressSync(
      [stateAddress.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), ORCA_MINT_ADDRESS.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    console.log('📋 Account Addresses:');
    console.log(`State Account: ${stateAddress.toString()}`);
    console.log(`Vault Account: ${vaultAddress.toString()}`);
    console.log(`xORCA Mint: ${XORCA_MINT_ADDRESS.toString()}`);
    console.log(`ORCA Mint: ${ORCA_MINT_ADDRESS.toString()}`);
    console.log('');

    // Fetch state account data
    console.log('📊 Fetching state account data...');
    const stateAccountInfo = await connection.getAccountInfo(stateAddress);
    if (!stateAccountInfo) {
      console.error('❌ State account not found. Program may not be initialized.');
      return;
    }

    // Parse state data (assuming the state structure from the program)
    const stateData = stateAccountInfo.data;

    // Read the state fields (adjust offsets based on your actual state structure)
    // This is a simplified version - you may need to adjust based on your actual state layout
    const coolDownPeriodS = stateData.readBigInt64LE(8); // Assuming cool_down_period_s is at offset 8
    const escrowedOrcaAmount = stateData.readBigUInt64LE(16); // Assuming escrowed_orca_amount is at offset 16

    console.log('📈 State Information:');
    console.log(`Cool Down Period: ${coolDownPeriodS.toString()} seconds`);
    console.log(`Escrowed ORCA Amount: ${escrowedOrcaAmount.toString()}`);
    console.log('');

    // Fetch vault account data
    console.log('💰 Fetching vault account data...');
    try {
      const vaultAccount = await getAccount(connection, vaultAddress);
      const vaultAmount = vaultAccount.amount;

      console.log('🏦 Vault Information:');
      console.log(`Total ORCA in Vault: ${vaultAmount.toString()}`);
      console.log(`Escrowed ORCA: ${escrowedOrcaAmount.toString()}`);

      const nonEscrowedAmount = vaultAmount - escrowedOrcaAmount;
      console.log(`Non-Escrowed ORCA: ${nonEscrowedAmount.toString()}`);
      console.log('');

      // Fetch xORCA mint supply
      console.log('🪙 Fetching xORCA mint supply...');
      const xorcaMint = await getMint(connection, XORCA_MINT_ADDRESS);
      const xorcaSupply = xorcaMint.supply;

      console.log('📊 xORCA Information:');
      console.log(`xORCA Total Supply: ${xorcaSupply.toString()}`);
      console.log('');

      // Calculate exchange rate
      console.log('📈 Calculating Exchange Rate...');

      // Add virtual amounts for DOS protection (matching the Rust implementation)
      const numerator = nonEscrowedAmount + VIRTUAL_NON_ESCROWED_ORCA_AMOUNT;
      const denominator = xorcaSupply + VIRTUAL_XORCA_SUPPLY;

      const exchangeRate = Number(numerator) / Number(denominator);
      const inverseExchangeRate = Number(denominator) / Number(numerator);

      console.log('🔄 Exchange Rates:');
      console.log(
        `ORCA → xORCA Rate: ${exchangeRate.toFixed(10)} (1 ORCA = ${exchangeRate.toFixed(10)} xORCA)`
      );
      console.log(
        `xORCA → ORCA Rate: ${inverseExchangeRate.toFixed(10)} (1 xORCA = ${inverseExchangeRate.toFixed(10)} ORCA)`
      );
      console.log('');

      // Display raw values for debugging
      console.log('🔍 Raw Exchange Rate Values:');
      console.log(`Numerator (Non-Escrowed + Virtual): ${numerator.toString()}`);
      console.log(`Denominator (xORCA Supply + Virtual): ${denominator.toString()}`);
      console.log('');

      // Summary
      console.log('📋 Summary:');
      console.log(`• Total ORCA in Vault: ${vaultAmount.toString()}`);
      console.log(`• Escrowed ORCA: ${escrowedOrcaAmount.toString()}`);
      console.log(`• Non-Escrowed ORCA: ${nonEscrowedAmount.toString()}`);
      console.log(`• xORCA Supply: ${xorcaSupply.toString()}`);
      console.log(`• Exchange Rate (ORCA→xORCA): ${exchangeRate.toFixed(10)}`);
      console.log(`• Exchange Rate (xORCA→ORCA): ${inverseExchangeRate.toFixed(10)}`);
    } catch (error) {
      console.error('❌ Error fetching vault account:', error);
      console.log('💡 The vault account may not exist yet. Try initializing the program first.');
    }
  } catch (error) {
    console.error('❌ Error fetching program status:', error);
    process.exit(1);
  }
}

main().catch(console.error);
