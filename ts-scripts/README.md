# xORCA TypeScript Scripts

This directory contains TypeScript scripts for interacting with the xORCA staking program on Solana devnet. These scripts provide a complete interface for all staking operations, program management, and monitoring functionality.

## 🚀 Quick Start

```bash
# Install dependencies
yarn install

# Run any script
yarn initialize <args>
yarn stake <args>
yarn unstake <args>
yarn withdraw <args>
yarn transfer-orca <args>
yarn transfer-orca-to-vault <args>
yarn update-mint-authority <args>
yarn pending-withdraws <args>
yarn status
yarn set <args>
yarn generate-keypair
yarn upload-image
```

## 📁 Project Structure

```
ts-scripts/
├── constants.ts          # Centralized constants and configuration
├── initialize.ts        # Initialize the staking program
├── stake.ts            # Stake ORCA tokens
├── unstake.ts          # Unstake xORCA tokens
├── withdraw.ts         # Withdraw ORCA tokens after cooldown
├── transfer-orca.ts    # Transfer ORCA tokens between accounts
├── transfer-orca-to-vault.ts # Transfer ORCA directly to vault PDA
├── update-xorca-mint-authority.ts # Update xORCA mint authority
├── pending-withdraws.ts # Check pending withdraws for a staker
├── status.ts           # Check program status and exchange rates
├── set.ts              # Update program parameters (cooldown, authority)
├── generate-keypair.ts # Generate new keypairs for testing
├── upload-image.ts     # Upload images to IPFS
├── keypairs/           # Keypair files (gitignored for security)
└── README.md           # This file
```

## Scripts

### 1. Status Script (`status.ts`)

This script displays the current state of the xORCA staking program without requiring any CLI arguments.

#### Usage

```bash
yarn status
```

#### What the script displays

- **Account Addresses**: State account, vault account, mint addresses
- **State Information**: Cool down period, escrowed ORCA amount
- **Vault Information**: Total ORCA in vault, escrowed vs non-escrowed amounts
- **xORCA Information**: Total xORCA supply
- **Exchange Rates**: Current ORCA ↔ xORCA conversion rates (with virtual amounts for DOS protection)
- **Summary**: Key metrics at a glance

#### Example Output

```
🔍 Fetching xORCA Staking Program Status...
============================================================
📋 Account Addresses:
State Account: 8RcfsSZakW3JmuYUuz6UZoN6zfGpyhsdNRSdPhqMUft8
Vault Account: FMZaievvLCmkuxS2E6XTgkWXUejfPXnr7ESYFHkyFr5J
xORCA Mint: Cz1vQJVwpD1Gzy4PEw6yxKNq7MxbPA8Ac7wBrieUmdGz
ORCA Mint: 51ipJjMd3aSxyy97du4MDU61GQaUCgehVmyHjfojJpxH

📈 State Information:
Cool Down Period: 350000 seconds
Escrowed ORCA Amount: 1800

🏦 Vault Information:
Total ORCA in Vault: 500000
Escrowed ORCA: 1800
Non-Escrowed ORCA: 498200

📊 xORCA Information:
xORCA Total Supply: 150000

🔄 Exchange Rates:
ORCA → xORCA Rate: 3.3197868088 (1 ORCA = 3.3197868088 xORCA)
xORCA → ORCA Rate: 0.3012241622 (1 xORCA = 0.3012241622 ORCA)
```

### 2. Transfer ORCA to Vault Script (`transfer-orca-to-vault.ts`)

This script transfers ORCA tokens directly to the vault PDA (Program Derived Address) without going through the staking process.

#### Usage

```bash
yarn transfer-orca-to-vault <sender-keypair-path> <orca-amount>
```

#### Parameters

- `sender-keypair-path`: Path to the JSON keypair file of the sender
- `orca-amount`: Amount of ORCA to transfer (in smallest units, 6 decimals)

#### Example

```bash
# Transfer 100,000 ORCA (0.1 ORCA) to the vault
yarn transfer-orca-to-vault keypairs/deployer.json 100000
```

#### What the script does

1. Loads the sender keypair
2. Derives the vault PDA address using the state account and ORCA mint
3. Checks sender's ORCA balance
4. Verifies the vault account exists
5. Creates a direct transfer instruction from sender to vault
6. Signs and sends the transaction
7. Confirms the transfer and displays final balances

#### Important Notes

- This transfers ORCA directly to the vault without minting xORCA tokens
- The vault must already exist (program must be initialized)
- This bypasses the normal staking flow
- Use with caution as it may affect the program's accounting

### 3. Pending Withdraws Script (`pending-withdraws.ts`)

This script reports on all pending withdraws for a given staker address, including withdrawal readiness and timing information.

#### Usage

```bash
yarn pending-withdraws <staker-public-key>
```

#### Parameters

- `staker-public-key`: The public key of the staker to check pending withdraws for

#### Example

```bash
# Check pending withdraws for a staker
yarn pending-withdraws BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq
```

#### What the script displays

- **Pending Withdraw Details**: Each pending withdraw with its index, address, and xORCA amount
- **Timing Information**: Creation timestamp, time elapsed, and cooldown status
- **Withdrawal Status**: Whether each withdraw is ready or still in cooldown
- **Time Calculations**: How long until ready (if in cooldown) or how long since ready (if ready)
- **Summary**: Total counts and amounts of ready vs cooldown withdraws

#### Example Output

```
🔍 Fetching pending withdraws for staker...
Staker: BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq
================================================================================
State account: 8RcfsSZakW3JmuYUuz6UZoN6zfGpyhsdNRSdPhqMUft8
📊 Fetching state account data...
Cool down period: 350000 seconds

🔍 Searching for pending withdraw accounts...

📋 Found 2 pending withdraws
================================================================================

1. Pending Withdraw #0
   Address: 7xK8vQ9mN2pL3rT5sW6uY1zA4bC7dE9fG2hJ5kM8nP
   xORCA Amount: 1000000000
   Created: 2024-01-15T10:30:00.000Z
   Time Elapsed: 2d 5h 30m
   ✅ Status: READY TO WITHDRAW
   ⏰ Ready for: 1d 2h 15m

2. Pending Withdraw #1
   Address: 9yL7wR8nO3qM4sT6uV2xZ5bD8eF1gH4jK6lN9oQ3rS
   xORCA Amount: 500000000
   Created: 2024-01-16T14:45:00.000Z
   Time Elapsed: 1d 1h 15m
   ⏳ Status: COOLDOWN ACTIVE
   ⏰ Time Remaining: 1d 2h 45m

📊 Summary:
Total Pending Withdraws: 2
Ready to Withdraw: 1
In Cooldown: 1
Total xORCA Ready: 1000000000
Total xORCA in Cooldown: 500000000
```

### 4. Initialize Script (`initialize.ts`)

This script initializes the xORCA staking program by calling the initialize instruction.

#### Usage

```bash
tsx initialize.ts <deployer-keypair-path> <update-authority-keypair-path> <cool-down-period-seconds>
```

#### Parameters

- `deployer-keypair-path`: Path to the JSON keypair file of the deployer (must match DEPLOYER_ADDRESS)
- `update-authority-keypair-path`: Path to the JSON keypair file of the update authority
- `cool-down-period-seconds`: Cool down period in seconds for unstaking

#### Example

```bash
# Initialize with 1 hour (3600 seconds) cool down period
tsx initialize.ts keypairs/deployer.json keypairs/authority.json 3600
```

#### What the script does

1. Loads the deployer and update authority keypairs
2. Verifies the deployer address matches the expected DEPLOYER_ADDRESS
3. Derives all required account addresses:
   - State account (PDA using "state" seed)
   - Vault account (ATA for the state + ORCA mint)
4. Creates an initialize instruction with the specified cool down period
5. Signs and sends the transaction with both keypairs
6. Confirms the transaction and displays the result

#### Required Accounts

The script automatically derives these accounts:

- `payerAccount` - The deployer (signer, must match DEPLOYER_ADDRESS)
- `updateAuthorityAccount` - The update authority (signer)
- `stateAccount` - State PDA
- `vaultAccount` - Vault ATA
- `xorcaMintAccount` - xORCA mint
- `orcaMintAccount` - ORCA mint
- `systemProgramAccount` - System program
- `tokenProgramAccount` - Token program
- `associatedTokenProgramAccount` - Associated Token program

### 2. Update xORCA Mint Authority Script (`update-xorca-mint-authority.ts`)

This script updates the mint authority for the xORCA token to point to the state account. This is required before the initialize instruction can succeed.

#### Usage

```bash
tsx update-xorca-mint-authority.ts <current-authority-keypair-path>
```

#### Parameters

- `current-authority-keypair-path`: Path to the JSON keypair file of the current mint authority

#### Example

```bash
# Update mint authority to state account
tsx update-xorca-mint-authority.ts keypairs/authority.json
```

#### What the script does

1. Loads the current authority keypair
2. Derives the state account address using the xORCA staking program ID and "state" seed
3. Checks the current mint authority and mint data
4. Creates a set authority instruction to change the mint authority to the state account
5. Signs and sends the transaction
6. Verifies the authority was successfully updated

### 3. Stake Script (`stake.ts`)

This script allows users to stake ORCA tokens and receive xORCA tokens in return.

#### Usage

```bash
tsx stake.ts <staker-keypair-path> <orca-amount>
```

#### Parameters

- `staker-keypair-path`: Path to the JSON keypair file of the staker
- `orca-amount`: Amount of ORCA to stake (in smallest units, 6 decimals)

#### Example

```bash
# Stake 1 ORCA (1000000 smallest units)
tsx stake.ts keypairs/staker.json 1000000
```

#### What the script does

1. Loads the staker keypair
2. Derives all required account addresses:
   - State account (PDA using "state" seed)
   - Vault account (ATA for state + ORCA mint)
   - Staker's ORCA ATA
   - Staker's xORCA ATA
3. Checks staker's ORCA balance and validates sufficient funds
4. Creates a stake instruction with the specified ORCA amount
5. Signs and sends the transaction
6. Displays final balances and staking results

#### Required Accounts

The script automatically derives these accounts:

- `stakerAccount` - The staker (signer)
- `vaultAccount` - Vault ATA (state + ORCA mint)
- `stakerOrcaAta` - Staker's ORCA ATA
- `stakerXorcaAta` - Staker's xORCA ATA
- `xorcaMintAccount` - xORCA mint
- `stateAccount` - State PDA
- `orcaMintAccount` - ORCA mint
- `tokenProgramAccount` - Token program

### 4. Transfer ORCA Script (`transfer-orca.ts`)

This script allows users to transfer ORCA tokens from one account to another.

#### Usage

```bash
tsx transfer-orca.ts <sender-keypair-path> <recipient-publickey> <orca-amount>
```

#### Parameters

- `sender-keypair-path`: Path to the JSON keypair file of the sender
- `recipient-publickey`: Public key of the recipient account
- `orca-amount`: Amount of ORCA to transfer (in smallest units, 6 decimals)

#### Example

```bash
# Transfer 1 ORCA (1000000 smallest units) to a recipient
tsx transfer-orca.ts keypairs/sender.json <recipient-publickey> 1000000
```

#### What the script does

1. Loads the sender keypair
2. Derives sender's and recipient's ORCA ATAs
3. Checks sender's ORCA balance and validates sufficient funds
4. Checks if recipient's ORCA ATA exists, creates it if necessary
5. Creates a transfer instruction with the specified ORCA amount
6. Signs and sends the transaction (including ATA creation if needed)
7. Displays final balances and transfer results

#### Required Accounts

The script automatically derives these accounts:

- `senderOrcaAta` - Sender's ORCA ATA
- `recipientOrcaAta` - Recipient's ORCA ATA
- `senderKeypair` - Sender's keypair (signer)
- `recipientPublicKey` - Recipient's public key

#### Prerequisites

- Sender must have sufficient ORCA balance
- Sender must have sufficient SOL for transaction fees (including ATA creation if needed)
- Both accounts must be on the same network (devnet)
- Recipient's ORCA ATA will be created automatically if it doesn't exist

## Scripts

### 1. Initialize Staking Program (`initialize.ts`)

Initializes the xORCA staking program with the required state and vault accounts.

```bash
# Initialize the staking program
tsx initialize.ts <deployer-keypair-path> <update-authority-keypair-path> <cool-down-period-seconds>
```

#### Example

```bash
# Initialize with 1 hour cool down period
tsx initialize.ts keypairs/deployer.json keypairs/authority.json 3600
```

#### What the script does

1. Loads the deployer and update authority keypairs
2. Derives the state account PDA using the program ID and "state" seed
3. Derives the vault account ATA for the state account + ORCA mint
4. Creates the initialize instruction with all required accounts
5. Signs and sends the transaction with both keypairs as signers
6. Displays the transaction signature and explorer link

#### Required Accounts

The script automatically derives these accounts:

- `stateAccount` - State PDA (program + "state" seed)
- `vaultAccount` - Vault ATA (state + ORCA mint)
- `deployerKeypair` - Deployer keypair (signer)
- `updateAuthorityKeypair` - Update authority keypair (signer)

#### Prerequisites

- Deployer must have sufficient SOL for transaction fees
- Both keypairs must be valid and have SOL for fees
- Program must be deployed to the specified program ID

### 2. Update xORCA Mint Authority (`update-xorca-mint-authority.ts`)

Updates the mint authority of the xORCA token to the state account.

```bash
# Update xORCA mint authority to state account
tsx update-xorca-mint-authority.ts <current-authority-keypair-path>
```

#### Example

```bash
# Update mint authority using deployer keypair
tsx update-xorca-mint-authority.ts keypairs/deployer.json
```

#### What the script does

1. Loads the current authority keypair
2. Derives the state account address using the program ID
3. Creates a set authority instruction to transfer mint authority to the state account
4. Signs and sends the transaction
5. Verifies the authority was updated successfully

#### Required Accounts

The script automatically derives these accounts:

- `stateAccount` - State PDA (program + "state" seed)
- `currentAuthorityKeypair` - Current mint authority keypair (signer)

#### Prerequisites

- Current authority must have sufficient SOL for transaction fees
- Current authority must be the current mint authority of xORCA token
- xORCA token must exist and be properly configured

### 3. Stake ORCA (`stake.ts`)

Stakes ORCA tokens to receive xORCA tokens.

```bash
# Stake ORCA tokens
tsx stake.ts <staker-keypair-path> <orca-amount>
```

#### Example

```bash
# Stake 1 ORCA (1000000 smallest units)
tsx stake.ts keypairs/staker.json 1000000
```

#### What the script does

1. Loads the staker keypair
2. Derives all required PDAs and ATAs
3. Checks staker's ORCA balance and validates sufficient funds
4. Checks if staker's xORCA ATA exists, creates it if necessary
5. Creates a stake instruction with the specified ORCA amount
6. Signs and sends the transaction (including ATA creation if needed)
7. Displays final balances and stake results

#### Required Accounts

The script automatically derives these accounts:

- `stateAccount` - State PDA (program + "state" seed)
- `vaultAccount` - Vault ATA (state + ORCA mint)
- `stakerOrcaAta` - Staker's ORCA ATA
- `stakerXorcaAta` - Staker's xORCA ATA
- `stakerKeypair` - Staker's keypair (signer)

#### Prerequisites

- Staker must have sufficient ORCA balance
- Staker must have sufficient SOL for transaction fees (including ATA creation if needed)
- Staking program must be initialized
- xORCA mint authority must be set to the state account

### 4. Unstake xORCA (`unstake.ts`)

Unstakes xORCA tokens to receive ORCA tokens back.

```bash
# Unstake xORCA tokens
tsx unstake.ts <staker-keypair-path> <xorca-amount> <withdraw-index>
```

#### Example

```bash
# Unstake 1 xORCA (1000000 smallest units) with withdraw index 0
tsx unstake.ts keypairs/staker.json 1000000 0
```

#### What the script does

1. Loads the staker keypair
2. Derives all required PDAs and ATAs
3. Checks staker's xORCA balance and validates sufficient funds
4. Checks if staker's ORCA ATA exists, creates it if necessary
5. Derives the pending withdraw account using the specified withdraw index
6. Creates an unstake instruction with the specified xORCA amount and withdraw index
7. Signs and sends the transaction (including ATA creation if needed)
8. Displays final balances and unstake results

#### Required Accounts

The script automatically derives these accounts:

- `stateAccount` - State PDA (program + "state" seed)
- `vaultAccount` - Vault ATA (state + ORCA mint)
- `stakerOrcaAta` - Staker's ORCA ATA
- `stakerXorcaAta` - Staker's xORCA ATA
- `stakerKeypair` - Staker's keypair (signer)

#### Prerequisites

- Staker must have sufficient xORCA balance
- Staker must have sufficient SOL for transaction fees (including ATA creation if needed)
- Staking program must be initialized
- Vault must have sufficient ORCA tokens for unstaking

### 5. Withdraw from Pending Withdraw (`withdraw.ts`)

Withdraws ORCA tokens from a pending withdraw account after the cooldown period has elapsed.

```bash
# Withdraw from pending withdraw account
tsx withdraw.ts <staker-keypair-path> <withdraw-index>
```

#### Example

```bash
# Withdraw from pending withdraw account with index 0
tsx withdraw.ts keypairs/staker.json 0
```

#### What the script does

1. Loads the staker keypair
2. Derives all required PDAs and ATAs
3. Checks if the pending withdraw account exists
4. Checks if staker's ORCA ATA exists, creates it if necessary
5. Creates a withdraw instruction with the specified withdraw index
6. Signs and sends the transaction (including ATA creation if needed)
7. Handles cooldown period errors gracefully
8. Displays final balances and withdrawal results

#### Required Accounts

The script automatically derives these accounts:

- `stateAccount` - State PDA (program + "state" seed)
- `pendingWithdrawAccount` - Pending withdraw PDA (program + "pending_withdraw" + staker + withdraw_index)
- `vaultAccount` - Vault ATA (state + ORCA mint)
- `stakerOrcaAta` - Staker's ORCA ATA
- `stakerKeypair` - Staker's keypair (signer)

#### Prerequisites

- Pending withdraw account must exist (created by unstake)
- Cooldown period must have elapsed
- Staker must have sufficient SOL for transaction fees (including ATA creation if needed)
- Staking program must be initialized

#### Important Notes

- **Cooldown Period**: The transaction will fail if the cooldown period has not elapsed
- **Expected Failure**: This is normal behavior - the script handles cooldown errors gracefully
- **Two-Step Process**: Unstake first, then withdraw after cooldown period

### 6. Transfer ORCA (`transfer-orca.ts`)

Transfers ORCA tokens between accounts.

```bash
# Transfer ORCA tokens
tsx transfer-orca.ts <sender-keypair-path> <recipient-publickey> <orca-amount>
```

#### Example

```bash
# Transfer 1 ORCA (1000000 smallest units) to a recipient
tsx transfer-orca.ts keypairs/sender.json <recipient-publickey> 1000000
```

#### What the script does

1. Loads the sender keypair
2. Derives sender's and recipient's ORCA ATAs
3. Checks sender's ORCA balance and validates sufficient funds
4. Checks if recipient's ORCA ATA exists, creates it if necessary
5. Creates a transfer instruction with the specified ORCA amount
6. Signs and sends the transaction (including ATA creation if needed)
7. Displays final balances and transfer results

#### Required Accounts

The script automatically derives these accounts:

- `senderOrcaAta` - Sender's ORCA ATA
- `recipientOrcaAta` - Recipient's ORCA ATA
- `senderKeypair` - Sender's keypair (signer)
- `recipientPublicKey` - Recipient's public key

#### Prerequisites

- Sender must have sufficient ORCA balance
- Sender must have sufficient SOL for transaction fees (including ATA creation if needed)
- Both accounts must be on the same network (devnet)
- Recipient's ORCA ATA will be created automatically if it doesn't exist

### 7. Set Script (`set.ts`)

This script allows the update authority to modify program parameters such as the cooldown period and update authority.

#### Usage

```bash
# Update cooldown period
yarn set <authority-keypair-path> update-cooldown <seconds>

# Update authority
yarn set <authority-keypair-path> update-authority <new-authority-publickey>
```

#### Examples

```bash
# Set cooldown to 1 hour (3600 seconds)
yarn set keypairs/authority.json update-cooldown 3600

# Update authority to a new address
yarn set keypairs/authority.json update-authority BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq
```

### 8. Generate Keypair Script (`generate-keypair.ts`)

This script generates new keypairs for testing purposes.

#### Usage

```bash
yarn generate-keypair <output-filename>
```

#### Example

```bash
# Generate a new keypair
yarn generate-keypair keypairs/new-user.json
```

### 9. Upload Image Script (`upload-image.ts`)

This script uploads images to IPFS using Irys for decentralized storage.

#### Usage

```bash
yarn upload-image <image-path>
```

#### Example

```bash
# Upload an image to IPFS
yarn upload-image ./images/logo.png
```

## Prerequisites

- Node.js and npm/yarn installed
- Valid keypair files for the operations
- The staking program must be deployed
- The xORCA mint must be created and configured properly
- For image uploads: Irys network access

## Network

The scripts are configured to use Solana devnet by default. To change the network, modify the `RPC_URL` constant in the scripts.
