#!/usr/bin/env tsx

import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import Irys from '@irys/sdk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Upload an image to Irys
 * Usage: tsx upload-image.ts <image-path> <keypair-path> [network]
 */

async function main() {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.error('Usage: tsx upload-image.ts <image-path> <keypair-path> [network]');
    console.error('Example: tsx upload-image.ts ./image.png ./keypairs/deployer.json mainnet');
    console.error('Networks: mainnet, devnet (default: mainnet)');
    process.exit(1);
  }

  const imagePath = args[0];
  const keypairPath = args[1];
  const network = args[2] || 'mainnet';

  // Validate network
  if (!['mainnet', 'devnet'].includes(network)) {
    console.error('❌ Invalid network. Use "mainnet" or "devnet"');
    process.exit(1);
  }

  try {
    // Check if image file exists
    if (!existsSync(imagePath)) {
      console.error(`❌ Image file not found: ${imagePath}`);
      process.exit(1);
    }

    // Check if keypair file exists
    if (!existsSync(keypairPath)) {
      console.error(`❌ Keypair file not found: ${keypairPath}`);
      process.exit(1);
    }

    // Read and parse keypair file
    console.log(`🔑 Loading keypair from: ${keypairPath}`);
    const keypairData = JSON.parse(readFileSync(keypairPath, 'utf8'));

    if (!Array.isArray(keypairData) || keypairData.length !== 64) {
      console.error('❌ Invalid keypair format. Expected array of 64 numbers.');
      process.exit(1);
    }

    // Convert keypair array to Uint8Array
    const privateKey = new Uint8Array(keypairData);
    console.log(`✅ Keypair loaded successfully`);

    // Read the image file
    console.log(`📁 Reading image: ${imagePath}`);
    const imageBuffer = readFileSync(imagePath);
    console.log(`✅ Image loaded: ${imageBuffer.length} bytes`);

    // Initialize Irys
    console.log(`🌐 Connecting to Irys ${network}...`);
    const irys = new Irys({
      url: network === 'mainnet' ? 'https://node1.irys.xyz' : 'https://devnet.irys.xyz',
      token: 'solana',
      key: privateKey,
    });

    // Upload the image
    console.log('📤 Uploading image to Irys...');
    const uploadResponse = await irys.upload(imageBuffer, {
      tags: [
        { name: 'Content-Type', value: getContentType(imagePath) },
        { name: 'App-Name', value: 'xORCA' },
        { name: 'Upload-Date', value: new Date().toISOString() },
      ],
    });

    console.log('✅ Upload successful!');
    console.log(`🔗 Transaction ID: ${uploadResponse.id}`);
    console.log(
      `🌐 View on Irys: https://${network === 'mainnet' ? 'gateway' : 'devnet'}.irys.xyz/${uploadResponse.id}`
    );
    console.log(`📊 Size: ${imageBuffer.length} bytes`);
    console.log(`💰 Upload completed successfully`);
  } catch (error) {
    if (error instanceof Error) {
      console.error('❌ Error uploading image:', error.message);
    } else {
      console.error('❌ Error uploading image:', error);
    }
    process.exit(1);
  }
}

function getContentType(filePath: string): string {
  const extension = filePath.split('.').pop()?.toLowerCase();

  switch (extension) {
    case 'png':
      return 'image/png';
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg';
    case 'gif':
      return 'image/gif';
    case 'webp':
      return 'image/webp';
    case 'svg':
      return 'image/svg+xml';
    case 'bmp':
      return 'image/bmp';
    case 'tiff':
    case 'tif':
      return 'image/tiff';
    case 'ico':
      return 'image/x-icon';
    default:
      return 'application/octet-stream';
  }
}

main();
