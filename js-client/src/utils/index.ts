import {
  getAccountDiscriminatorEncoder,
  getPendingWithdrawDecoder,
  getStateDecoder,
  getTokenMintDecoder,
  PENDING_WITHDRAW_DISCRIMINATOR,
  PendingWithdraw,
  State,
  STATE_DISCRIMINATOR,
  XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
} from '../generated';
import {
  getBase64Encoder,
  Rpc,
  Address,
  GetProgramAccountsMemcmpFilter,
  VariableSizeDecoder,
  Account,
  GetMultipleAccountsApi,
  getBase58Decoder,
  Base58EncodedBytes,
  GetProgramAccountsApi,
  getProgramDerivedAddress,
  ProgramDerivedAddress,
  GetAccountInfoApi,
} from '@solana/kit';
import { getAddressEncoder } from '@solana/addresses';
import { getTokenDecoder, getMintDecoder } from '@solana-program/token';

const TOKEN_PROGRAM_ADDRESS = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA' as Address;
const ASSOCIATED_TOKEN_PROGRAM_ADDRESS = 'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL' as Address;
const ORCA_MINT_ADDRESS = 'orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE' as Address;
const XORCA_MINT_ADDRESS = 'xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N' as Address; // TODO: update this

export async function findStateAddress(): Promise<ProgramDerivedAddress> {
  return await getProgramDerivedAddress({
    programAddress: XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
    seeds: [new TextEncoder().encode('state')],
  });
}

export async function findPendingWithdrawAddress(
  unstaker: Address,
  withdrawIndex: number
): Promise<ProgramDerivedAddress> {
  const addressEncoder = getAddressEncoder();
  const unstakerBytes = addressEncoder.encode(unstaker);

  return await getProgramDerivedAddress({
    programAddress: XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
    seeds: [
      new TextEncoder().encode('pending_withdraw'),
      unstakerBytes,
      new Uint8Array([withdrawIndex]),
    ],
  });
}

export async function findVaultAddress(
  state: Address,
  tokenProgram: Address,
  orcaMint: Address
): Promise<ProgramDerivedAddress> {
  const addressEncoder = getAddressEncoder();
  return await getProgramDerivedAddress({
    programAddress: ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
    seeds: [
      addressEncoder.encode(state),
      addressEncoder.encode(tokenProgram),
      addressEncoder.encode(orcaMint),
    ],
  });
}

export async function fetchDecodedProgramAccounts<T extends object>(
  rpc: Rpc<GetProgramAccountsApi>,
  programAddress: Address,
  filters: GetProgramAccountsMemcmpFilter[],
  decoder: VariableSizeDecoder<T>
): Promise<Account<T>[]> {
  const accountInfos = await rpc
    .getProgramAccounts(programAddress, {
      encoding: 'base64',
      filters,
    })
    .send();
  const encoder = getBase64Encoder();
  const datas = accountInfos.map((x) => encoder.encode(x.account.data[0]));
  const decoded = datas.map((x) => decoder.decode(x));
  return decoded.map((data, i) => ({
    ...accountInfos[i].account,
    address: accountInfos[i].pubkey,
    programAddress: programAddress,
    data,
  }));
}

export async function fetchState(
  rpc: Rpc<GetMultipleAccountsApi & GetProgramAccountsApi>
): Promise<State> {
  const discriminator = getBase58Decoder().decode(
    getAccountDiscriminatorEncoder().encode(STATE_DISCRIMINATOR)
  );
  let filters: GetProgramAccountsMemcmpFilter[] = [
    {
      memcmp: {
        offset: 0n,
        bytes: discriminator as Base58EncodedBytes,
        encoding: 'base58',
      },
    },
  ];
  const state = (
    await fetchDecodedProgramAccounts(
      rpc,
      XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
      filters,
      getStateDecoder()
    )
  )[0];
  return state.data;
}

export async function fetchStateAccountCoolDownPeriodS(
  rpc: Rpc<GetMultipleAccountsApi & GetProgramAccountsApi>
): Promise<bigint> {
  const state = await fetchState(rpc);
  return state.coolDownPeriodS;
}

export async function fetchPendingWithdrawsForStaker(
  rpc: Rpc<GetMultipleAccountsApi & GetProgramAccountsApi>,
  staker: Address
): Promise<PendingWithdraw[]> {
  const discriminator = getBase58Decoder().decode(
    getAccountDiscriminatorEncoder().encode(PENDING_WITHDRAW_DISCRIMINATOR)
  );
  const encodedStaker = staker as unknown as Base58EncodedBytes;
  let filters: GetProgramAccountsMemcmpFilter[] = [
    {
      memcmp: {
        offset: 0n,
        bytes: discriminator as Base58EncodedBytes,
        encoding: 'base58',
      },
    },
    {
      memcmp: {
        offset: 8n,
        bytes: encodedStaker,
        encoding: 'base58',
      },
    },
  ];
  const pendingWithdraws = await fetchDecodedProgramAccounts(
    rpc,
    XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
    filters,
    getPendingWithdrawDecoder()
  );
  return pendingWithdraws.map((x) => x.data);
}

export async function fetchVaultState(rpc: Rpc<GetAccountInfoApi>): Promise<{
  address: Address;
  owner: Address;
  mint: Address;
  amount: bigint;
}> {
  const statePda = await findStateAddress();
  const vaultPda = await findVaultAddress(
    statePda[0],
    TOKEN_PROGRAM_ADDRESS,
    ORCA_MINT_ADDRESS as Address
  );
  const accountInfo = await rpc.getAccountInfo(vaultPda[0], { encoding: 'base64' }).send();
  if (!accountInfo.value) {
    throw new Error('Vault ATA not found.');
  }
  const tokenDecoder = getTokenDecoder();
  const dataBytes = new Uint8Array(Buffer.from(accountInfo.value.data[0], 'base64'));
  const tokenAccount = tokenDecoder.decode(dataBytes);
  return {
    address: vaultPda[0],
    owner: tokenAccount.owner,
    mint: tokenAccount.mint,
    amount: tokenAccount.amount,
  };
}

export async function fetchXorcaMintSupply(rpc: Rpc<GetAccountInfoApi>): Promise<bigint> {
  const accountInfo = await rpc.getAccountInfo(XORCA_MINT_ADDRESS, { encoding: 'base64' }).send();
  if (!accountInfo.value) {
    throw new Error('xORCA mint not found.');
  }
  const mintDecoder = getMintDecoder();
  const dataBytes = new Uint8Array(Buffer.from(accountInfo.value.data[0], 'base64'));
  const mintAccount = mintDecoder.decode(dataBytes);
  return mintAccount.supply;
}

export async function getStakingExchangeRate(
  rpc: Rpc<GetMultipleAccountsApi & GetProgramAccountsApi & GetAccountInfoApi>
): Promise<{
  numerator: bigint;
  denominator: bigint;
}> {
  const state = await fetchState(rpc);
  const vault = await fetchVaultState(rpc);
  const numerator = vault.amount - state.escrowedOrcaAmount;
  const denominator = await fetchXorcaMintSupply(rpc);
  return {
    numerator,
    denominator,
  };
}
