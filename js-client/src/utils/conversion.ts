// Off-chain conversion helpers that mirror the on-chain staking math.

export const VIRTUAL_XORCA_SUPPLY = 100n;
export const VIRTUAL_NON_ESCROWED_ORCA_AMOUNT = 100n;

/**
 * Convert an ORCA amount to xORCA using the same virtual-offset defense as the on-chain program.
 * Returns the amount of xORCA to mint (integer division floors).
 */
export function convertOrcaToXorca(
  orcaAmountToConvert: bigint,
  nonEscrowedOrcaAmount: bigint,
  xorcaSupply: bigint
): bigint {
  if (xorcaSupply === 0n || nonEscrowedOrcaAmount === 0n) {
    return orcaAmountToConvert;
  }
  const xorcaSupplyWithVirtual = xorcaSupply + VIRTUAL_XORCA_SUPPLY;
  const nonEscrowedWithVirtual = nonEscrowedOrcaAmount + VIRTUAL_NON_ESCROWED_ORCA_AMOUNT;
  return (orcaAmountToConvert * xorcaSupplyWithVirtual) / nonEscrowedWithVirtual;
}

/**
 * Convert an xORCA amount to ORCA using the same virtual-offset defense as the on-chain program.
 * Returns the amount of ORCA to withdraw (integer division floors).
 * Throws if supply or non-escrowed amount is zero.
 */
export function convertXorcaToOrca(
  xorcaAmountToConvert: bigint,
  nonEscrowedOrcaAmount: bigint,
  xorcaSupply: bigint
): bigint {
  if (xorcaSupply === 0n || nonEscrowedOrcaAmount === 0n) {
    throw new Error('Arithmetic overflow or invalid inputs');
  }
  const xorcaSupplyWithVirtual = xorcaSupply + VIRTUAL_XORCA_SUPPLY;
  const nonEscrowedWithVirtual = nonEscrowedOrcaAmount + VIRTUAL_NON_ESCROWED_ORCA_AMOUNT;
  return (xorcaAmountToConvert * nonEscrowedWithVirtual) / xorcaSupplyWithVirtual;
}
