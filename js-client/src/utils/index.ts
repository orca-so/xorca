import {
  getPendingWithdrawDecoder,
  getStateDecoder,
  PendingWithdraw,
  State,
  XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
} from '../generated';
import {
  getBase64Encoder,
  Rpc,
  Address,
  GetMultipleAccountsApi,
  getProgramDerivedAddress,
  ProgramDerivedAddress,
  GetAccountInfoApi,
} from '@solana/kit';
import { getAddressEncoder } from '@solana/addresses';
import { getTokenDecoder, getMintDecoder } from '@solana-program/token';
export * from './conversion';

const TOKEN_PROGRAM_ADDRESS = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA' as Address;
const ASSOCIATED_TOKEN_PROGRAM_ADDRESS = 'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL' as Address;
const ORCA_MINT_ADDRESS = 'orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE' as Address;
const XORCA_MINT_ADDRESS = 'xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N' as Address; // TODO: update this

const DEFAULT_MAX_WITHDRAWALS_TO_SEARCH = 15;
const WITHDRAW_INDEX_MAX_UINT = 255;

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

export async function fetchStateAccountData(rpc: Rpc<GetMultipleAccountsApi>): Promise<State> {
  const [stateAddress] = await findStateAddress();
  const accounts = await rpc.getMultipleAccounts([stateAddress]).send();
  if (!accounts.value[0]) {
    throw new Error('State account not found.');
  }
  const encoder = getBase64Encoder();
  const dataBytes = encoder.encode(accounts.value[0].data[0]);
  const stateDecoder = getStateDecoder();
  return stateDecoder.decode(dataBytes);
}

export async function fetchStateAccountCoolDownPeriodS(
  rpc: Rpc<GetMultipleAccountsApi>
): Promise<bigint> {
  const state = await fetchStateAccountData(rpc);
  return state.coolDownPeriodS;
}

export async function fetchPendingWithdrawsForStaker(
  rpc: Rpc<GetMultipleAccountsApi>,
  staker: Address,
  maxWithdrawalsToSearch: number = DEFAULT_MAX_WITHDRAWALS_TO_SEARCH
): Promise<PendingWithdraw[]> {
  validateMaxWithdrawalsToSearch(maxWithdrawalsToSearch);
  const pendingWithdrawAddresses: Address[] = await Promise.all(
    Array.from({ length: maxWithdrawalsToSearch }, (_, index) =>
      findPendingWithdrawAddress(staker, index).then(([address]) => address)
    )
  );
  const accounts = await rpc
    .getMultipleAccounts(pendingWithdrawAddresses, { encoding: 'base64' })
    .send();
  const [pendingWithdrawDecoder, base64Encoder] = [getPendingWithdrawDecoder(), getBase64Encoder()];
  const pendingWithdraws = accounts.value
    .filter((account): account is NonNullable<typeof account> => Boolean(account))
    .map((account) => {
      const dataBytes = base64Encoder.encode(account.data[0]);
      try {
        return pendingWithdrawDecoder.decode(dataBytes);
      } catch (error) {
        console.error('Error decoding account data:', error);
        return null;
      }
    })
    .filter((data): data is PendingWithdraw => Boolean(data));
  return pendingWithdraws;
}

function validateMaxWithdrawalsToSearch(maxWithdrawalsToSearch: number): void {
  if (
    maxWithdrawalsToSearch < 0 ||
    maxWithdrawalsToSearch > WITHDRAW_INDEX_MAX_UINT ||
    maxWithdrawalsToSearch % 1 !== 0
  ) {
    throw new Error(
      `maxWithdrawalsToSearch must be between 0 and ${WITHDRAW_INDEX_MAX_UINT} and an integer`
    );
  }
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

export async function fetchStakingExchangeRate(
  rpc: Rpc<GetMultipleAccountsApi & GetAccountInfoApi>
): Promise<{
  numerator: bigint;
  denominator: bigint;
}> {
  const state = await fetchStateAccountData(rpc);
  const vault = await fetchVaultState(rpc);
  const numerator = vault.amount - state.escrowedOrcaAmount;
  const denominator = await fetchXorcaMintSupply(rpc);
  return {
    numerator,
    denominator,
  };
}
