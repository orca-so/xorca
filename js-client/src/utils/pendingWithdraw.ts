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
} from '@solana/kit';
import {
  getAccountDiscriminatorEncoder,
  getPendingWithdrawDecoder,
  PENDING_WITHDRAW_DISCRIMINATOR,
  PendingWithdraw,
  XORCA_STAKING_PROGRAM_PROGRAM_ADDRESS,
} from '../generated';

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
