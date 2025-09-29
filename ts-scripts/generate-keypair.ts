#!/usr/bin/env tsx

import { Keypair } from '@solana/web3.js';
import { writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Generate a keypair from a hex encoded private key and save it to the keypairs folder
 * Usage: tsx generate-keypair.ts <hex-private-key> [filename]
 */

function main() {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error('Usage: tsx generate-keypair.ts <hex-private-key> [filename]');
    console.error(
      'Example: tsx generate-keypair.ts 403f4b8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c'
    );
    process.exit(1);
  }

  const hexPrivateKey = args[0];
  const filename = args[1] || 'generated-keypair.json';

  try {
    // Decode hex private key
    const privateKeyBytes = Buffer.from(hexPrivateKey, 'hex');

    // Validate that we have the correct number of bytes (32 for private key, 64 for full keypair)
    if (privateKeyBytes.length === 32) {
      // If we have a 32-byte private key, create a keypair from it using fromSeed
      const keypair = Keypair.fromSeed(privateKeyBytes);
      const secretKeyArray = Array.from(keypair.secretKey);

      // Write to keypairs folder
      const keypairsDir = join(__dirname, 'keypairs');
      const filePath = join(keypairsDir, filename);

      writeFileSync(filePath, JSON.stringify(secretKeyArray));

      console.log(`✅ Keypair generated successfully!`);
      console.log(`📁 Saved to: ${filePath}`);
      console.log(`🔑 Public key: ${keypair.publicKey.toString()}`);
    } else if (privateKeyBytes.length === 64) {
      // If we have a 64-byte keypair, use it directly
      const keypair = Keypair.fromSecretKey(privateKeyBytes);
      const secretKeyArray = Array.from(keypair.secretKey);

      // Write to keypairs folder
      const keypairsDir = join(__dirname, 'keypairs');
      const filePath = join(keypairsDir, filename);

      writeFileSync(filePath, JSON.stringify(secretKeyArray));

      console.log(`✅ Keypair generated successfully!`);
      console.log(`📁 Saved to: ${filePath}`);
      console.log(`🔑 Public key: ${keypair.publicKey.toString()}`);
    } else {
      throw new Error(
        `Invalid private key length. Expected 32 or 64 bytes, got ${privateKeyBytes.length}`
      );
    }
  } catch (error) {
    if (error instanceof Error) {
      console.error('❌ Error generating keypair:', error.message);
    } else {
      console.error('❌ Error generating keypair:', error);
    }
    process.exit(1);
  }
}

main();
