import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import React from 'react';
import { useTx } from '@/hooks/useTx';
import { ApiContext } from '@/providers/ApiProvider';
import type { ApiPromise } from '@polkadot/api';
import type { ConnectionStatus } from '@/lib/types';

const VALID_TX_HASH = '0x' + 'a'.repeat(64);
const BLOCK_HASH_OBJ = {
  toHex: () => '0x' + 'b'.repeat(64),
};

function makeWrapper(api: ApiPromise | null) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(
      ApiContext.Provider,
      { value: { api, status: 'connected' as ConnectionStatus, blockNumber: null } },
      children,
    );
  };
}

function makeMockExtrinsic(hash: string, signed = true) {
  return {
    hash: { toHex: () => hash },
    method: {
      section: 'balances',
      method: 'transfer',
      args: [{ toHuman: () => '5Dest...' }],
      argsDef: { dest: 'AccountId' },
    },
    isSigned: signed,
    signer: signed ? { toString: () => '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY' } : null,
  };
}

describe('useTx', () => {
  it('stays loading when api is null', () => {
    const wrapper = makeWrapper(null);
    const { result } = renderHook(() => useTx(VALID_TX_HASH), { wrapper });
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('returns error immediately for invalid tx hash', async () => {
    const mockApi = {} as unknown as ApiPromise;
    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useTx('not-a-hash'), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toMatch(/Invalid transaction hash/);
    expect(result.current.data).toBeNull();
  });

  it('returns error for hash with wrong length', async () => {
    const mockApi = {} as unknown as ApiPromise;
    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useTx('0x' + 'a'.repeat(63)), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toMatch(/Invalid transaction hash/);
  });

  it('finds tx in block and returns full data', async () => {
    const mockEvents = [
      {
        phase: { applyExtrinsic: '0' },
        event: {
          section: 'system',
          method: 'ExtrinsicSuccess',
          data: {},
        },
      },
      {
        phase: { applyExtrinsic: '0' },
        event: {
          section: 'transactionPayment',
          method: 'TransactionFeePaid',
          data: { actualFee: '1000000000', tip: '0' },
        },
      },
    ];

    const mockApi = {
      rpc: {
        chain: {
          getHeader: vi.fn().mockResolvedValue({ number: { toNumber: () => 5 } }),
          getBlockHash: vi.fn().mockResolvedValue(BLOCK_HASH_OBJ),
          getBlock: vi.fn().mockResolvedValue({
            block: {
              extrinsics: [makeMockExtrinsic(VALID_TX_HASH)],
            },
          }),
        },
      },
      query: {
        system: {
          events: {
            at: vi.fn().mockResolvedValue({ toHuman: () => mockEvents }),
          },
        },
      },
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useTx(VALID_TX_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    const data = result.current.data;
    expect(data).not.toBeNull();
    expect(data?.hash).toBe(VALID_TX_HASH);
    expect(data?.section).toBe('balances');
    expect(data?.method).toBe('transfer');
    expect(data?.success).toBe(true);
    expect(data?.fee).toBe('1000000000');
    expect(data?.tip).toBe('0');
    expect(data?.events).toHaveLength(2);
    expect(data?.blockNumber).toBe(5);
    expect(data?.signer).toBeTruthy();
  });

  it('marks tx as failed when ExtrinsicFailed event present', async () => {
    const failEvents = [
      {
        phase: { applyExtrinsic: '0' },
        event: {
          section: 'system',
          method: 'ExtrinsicFailed',
          data: {},
        },
      },
    ];

    const mockApi = {
      rpc: {
        chain: {
          getHeader: vi.fn().mockResolvedValue({ number: { toNumber: () => 1 } }),
          getBlockHash: vi.fn().mockResolvedValue(BLOCK_HASH_OBJ),
          getBlock: vi.fn().mockResolvedValue({
            block: {
              extrinsics: [makeMockExtrinsic(VALID_TX_HASH, false)],
            },
          }),
        },
      },
      query: {
        system: {
          events: {
            at: vi.fn().mockResolvedValue({ toHuman: () => failEvents }),
          },
        },
      },
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useTx(VALID_TX_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data?.success).toBe(false);
    expect(result.current.data?.signer).toBeNull();
  });

  it('returns "not found" error when tx is in none of the blocks', async () => {
    const DIFFERENT_HASH = '0x' + 'f'.repeat(64);

    const mockApi = {
      rpc: {
        chain: {
          getHeader: vi.fn().mockResolvedValue({ number: { toNumber: () => 3 } }),
          getBlockHash: vi.fn().mockResolvedValue(BLOCK_HASH_OBJ),
          getBlock: vi.fn().mockResolvedValue({
            block: {
              // This block contains a different tx
              extrinsics: [makeMockExtrinsic(DIFFERENT_HASH)],
            },
          }),
        },
      },
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    // Search for VALID_TX_HASH which is NOT in any block
    const { result } = renderHook(() => useTx(VALID_TX_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    }, { timeout: 5000 });

    expect(result.current.error).toMatch(/Transaction not found/);
    expect(result.current.data).toBeNull();
  });

  it('returns error when API throws during search', async () => {
    const mockApi = {
      rpc: {
        chain: {
          getHeader: vi.fn().mockRejectedValue(new Error('network error')),
        },
      },
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useTx(VALID_TX_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('network error');
    expect(result.current.data).toBeNull();
  });
});
