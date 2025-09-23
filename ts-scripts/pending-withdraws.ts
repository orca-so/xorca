#!/usr/bin/env tsx

import { Connection, PublicKey } from '@solana/web3.js';
import { getAccount } from '@solana/spl-token';
import {
  ORCA_MINT_ADDRESS,
  XORCA_STAKING_PROGRAM_ID,
  RPC_URL,
  STATE_SEED,
  PENDING_WITHDRAW_SEED,
} from './constants';

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length < 1) {
  console.error('Usage: tsx pending-withdraws.ts <staker-public-key>');
  console.error('Example: tsx pending-withdraws.ts BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq');
  process.exit(1);
}

const stakerPublicKeyStr = args[0];
const stakerPublicKey = new PublicKey(stakerPublicKeyStr);

console.log('🔍 Fetching pending withdraws for staker...');
console.log(`Staker: ${stakerPublicKey.toString()}`);
console.log('='.repeat(80));

async function main() {
  try {
    // Initialize connection
    const connection = new Connection(RPC_URL, 'confirmed');

    // Derive state account address
    const [stateAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from(STATE_SEED)],
      XORCA_STAKING_PROGRAM_ID
    );

    console.log(`State account: ${stateAddress.toString()}`);

    // Fetch state account data to get cooldown period
    console.log('📊 Fetching state account data...');
    const stateAccountInfo = await connection.getAccountInfo(stateAddress);
    if (!stateAccountInfo) {
      console.error('❌ State account not found. Program may not be initialized.');
      return;
    }

    // Parse state data according to the Rust struct:
    // - discriminator: 1 byte
    // - padding1: 5 bytes
    // - bump: 1 byte
    // - vault_bump: 1 byte
    // - escrowed_orca_amount: 8 bytes (at offset 8)
    // - cool_down_period_s: 8 bytes (i64) at offset 16
    const stateData = stateAccountInfo.data;
    const escrowedOrcaAmount = stateData.readBigUInt64LE(8); // escrowed_orca_amount is at offset 8
    const coolDownPeriodS = stateData.readBigInt64LE(16); // cool_down_period_s is at offset 16

    console.log(`Cool down period: ${coolDownPeriodS.toString()} seconds`);
    console.log('');

    // Search for pending withdraw accounts
    console.log('🔍 Searching for pending withdraw accounts...');
    const maxWithdrawalsToSearch = 15; // Reasonable limit
    const pendingWithdraws = [];

    for (let withdrawIndex = 0; withdrawIndex < maxWithdrawalsToSearch; withdrawIndex++) {
      try {
        // Derive pending withdraw account address
        const [pendingWithdrawAddress] = PublicKey.findProgramAddressSync(
          [
            Buffer.from(PENDING_WITHDRAW_SEED),
            stakerPublicKey.toBuffer(),
            Buffer.from([withdrawIndex]),
          ],
          XORCA_STAKING_PROGRAM_ID
        );

        // Try to fetch the account
        const accountInfo = await connection.getAccountInfo(pendingWithdrawAddress);
        if (accountInfo) {
          // Parse pending withdraw data according to the Rust struct:
          // - discriminator: 1 byte
          // - padding1: 5 bytes
          // - bump: 1 byte
          // - withdraw_index: 1 byte
          // - unstaker: 32 bytes
          // - withdrawable_orca_amount: 8 bytes (u64)
          // - withdrawable_timestamp: 8 bytes (i64)
          const data = accountInfo.data;

          const discriminator = data.readUInt8(0);
          const bump = data.readUInt8(6);
          const storedWithdrawIndex = data.readUInt8(7);
          const unstaker = data.slice(8, 40); // 32 bytes
          const withdrawableOrcaAmount = data.readBigUInt64LE(40); // 8 bytes
          const withdrawableTimestamp = data.readBigInt64LE(48); // 8 bytes (signed)

          // Calculate current timestamp
          const currentTimestamp = BigInt(Math.floor(Date.now() / 1000));

          // withdrawableTimestamp is the future time when withdraw will be ready
          // So we need to check if current time >= withdrawableTimestamp
          const isReady = currentTimestamp >= withdrawableTimestamp;
          const timeRemaining = isReady ? 0n : withdrawableTimestamp - currentTimestamp;
          const timeSinceReady = isReady ? currentTimestamp - withdrawableTimestamp : 0n;

          // Calculate when the withdraw was created (withdrawableTimestamp - cooldown)
          const createdAtTimestamp = withdrawableTimestamp - coolDownPeriodS;
          const timeElapsed = currentTimestamp - createdAtTimestamp;

          pendingWithdraws.push({
            index: storedWithdrawIndex,
            address: pendingWithdrawAddress,
            xorcaAmount: withdrawableOrcaAmount,
            timestamp: withdrawableTimestamp,
            timeElapsed,
            timeRemaining,
            isReady,
            timeSinceReady,
          });
        }
      } catch (error) {
        // Account doesn't exist or parsing error, continue searching
        continue;
      }
    }

    console.log(`\n📋 Found ${pendingWithdraws.length} pending withdraws`);
    console.log('='.repeat(80));

    if (pendingWithdraws.length === 0) {
      console.log('❌ No pending withdraws found for this staker');
      return;
    }

    // Display each pending withdraw
    pendingWithdraws.forEach((withdraw, index) => {
      console.log(`\n${index + 1}. Pending Withdraw #${withdraw.index}`);
      console.log(`   Address: ${withdraw.address.toString()}`);
      console.log(`   xORCA Amount: ${withdraw.xorcaAmount.toString()}`);
      // Handle timestamp display safely - show when withdraw was created
      try {
        const createdAtTimestamp = withdraw.timestamp - coolDownPeriodS;
        const createdAtMs = Number(createdAtTimestamp) * 1000;
        if (isNaN(createdAtMs) || createdAtMs < 0) {
          console.log(`   Created: Invalid timestamp (${createdAtTimestamp.toString()})`);
        } else {
          console.log(`   Created: ${new Date(createdAtMs).toISOString()}`);
        }
      } catch (error) {
        console.log(`   Created: Invalid timestamp`);
      }
      console.log(`   Time Elapsed: ${formatDuration(Number(withdraw.timeElapsed))}`);

      if (withdraw.isReady) {
        console.log(`   ✅ Status: READY TO WITHDRAW`);
        console.log(`   ⏰ Ready for: ${formatDuration(Number(withdraw.timeSinceReady))}`);
      } else {
        console.log(`   ⏳ Status: COOLDOWN ACTIVE`);
        console.log(`   ⏰ Time Remaining: ${formatDuration(Number(withdraw.timeRemaining))}`);
      }
    });

    // Summary
    const readyWithdraws = pendingWithdraws.filter((w) => w.isReady);
    const cooldownWithdraws = pendingWithdraws.filter((w) => !w.isReady);

    console.log('\n📊 Summary:');
    console.log(`Total Pending Withdraws: ${pendingWithdraws.length}`);
    console.log(`Ready to Withdraw: ${readyWithdraws.length}`);
    console.log(`In Cooldown: ${cooldownWithdraws.length}`);

    if (readyWithdraws.length > 0) {
      const totalReadyXorca = readyWithdraws.reduce((sum, w) => sum + w.xorcaAmount, 0n);
      console.log(`Total xORCA Ready: ${totalReadyXorca.toString()}`);
    }

    if (cooldownWithdraws.length > 0) {
      const totalCooldownXorca = cooldownWithdraws.reduce((sum, w) => sum + w.xorcaAmount, 0n);
      console.log(`Total xORCA in Cooldown: ${totalCooldownXorca.toString()}`);
    }
  } catch (error) {
    console.error('❌ Error fetching pending withdraws:', error);
    process.exit(1);
  }
}

function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds} seconds`;
  } else if (seconds < 3600) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  } else if (seconds < 86400) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  } else {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    return `${days}d ${hours}h`;
  }
}

main().catch(console.error);
