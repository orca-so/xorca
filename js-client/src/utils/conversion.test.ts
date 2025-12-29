import { describe, expect, it } from 'vitest';
import { convertOrcaToXorca, convertXorcaToOrca } from './conversion';

describe('conversion utils', () => {
  it('convertOrcaToXorca handles nominal large values', () => {
    const out = convertOrcaToXorca(100_000_000_000n, 400_000_000_000n, 200_000_000_000n);
    // With virtual offsets: supply=200_000_000_100, non-escrowed=400_000_000_100 =>
    // 100_000_000_000 * 200_000_000_100 / 400_000_000_100 = 50_000_000_012 (floored)
    expect(out).toBe(50_000_000_012n);
  });

  it('convertXorcaToOrca handles nominal large values', () => {
    const out = convertXorcaToOrca(50_000_000_000n, 500_000_000_000n, 250_000_000_000n);
    // With virtual offsets: supply=250_000_000_100, non-escrowed=500_000_000_100 =>
    // 50_000_000_000 * 500_000_000_100 / 250_000_000_100 = 99_999_999_980 (floored)
    expect(out).toBe(99_999_999_980n);
  });

  it('convertXorcaToOrca throws on zero supply/non-escrowed', () => {
    expect(() => convertXorcaToOrca(1n, 0n, 0n)).toThrowError();
  });
});
