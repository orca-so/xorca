# xORCA TypeScript Scripts

This directory contains TypeScript scripts for interacting with the xORCA staking program on Solana devnet.

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
yarn update-mint-authority <args>
yarn status
```

## 📁 Project Structure

```
ts-scripts/
├── constants.ts          # Centralized constants and configuration
├── utils.ts             # Common utility functions
├── errors.ts            # Custom error classes
├── initialize.ts        # Initialize the staking program
├── stake.ts            # Stake ORCA tokens
├── unstake.ts          # Unstake xORCA tokens
├── withdraw.ts         # Withdraw ORCA tokens after cooldown
├── transfer-orca.ts    # Transfer ORCA tokens between accounts
├── update-xorca-mint-authority.ts # Update xORCA mint authority
├── status.ts           # Check program status and exchange rates
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

### 2. Initialize Script (`initialize.ts`)

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

## Prerequisites

- Node.js and npm/yarn installed
- Valid keypair files for the operations
- The staking program must be deployed
- The xORCA mint must be created and configured properly

## Network

The scripts are configured to use Solana devnet by default. To change the network, modify the `RPC_URL` constant in the scripts.
