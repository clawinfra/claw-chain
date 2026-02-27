import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import React from 'react';
import { useBlock } from '@/hooks/useBlock';
import { ApiContext } from '@/providers/ApiProvider';
import type { ApiPromise } from '@polkadot/api';
import type { ConnectionStatus } from '@/lib/types';

const BLOCK_HASH = '0x' + 'a'.repeat(64);
const PARENT_HASH = '0x' + 'b'.repeat(64);
const STATE_ROOT = '0x' + 'c'.repeat(64);
const EXTR_ROOT = '0x' + 'd'.repeat(64);
const TX_HASH = '0x' + 'e'.repeat(64);

function makeWrapper(api: ApiPromise | null) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(
      ApiContext.Provider,
      { value: { api, status: 'connected' as ConnectionStatus, blockNumber: null } },
      children,
    );
  };
}

function makeMockBlock(extrinsics: unknown[] = []) {
  return {
    block: {
      header: {
        number: { toNumber: () => 100 },
        parentHash: { toHex: () => PARENT_HASH },
        stateRoot: { toHex: () => STATE_ROOT },
        extrinsicsRoot: { toHex: () => EXTR_ROOT },
      },
      extrinsics,
    },
  };
}

describe('useBlock', () => {
  it('stays loading when api is null', () => {
    const wrapper = makeWrapper(null);
    const { result } = renderHook(() => useBlock(BLOCK_HASH), { wrapper });
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('returns error for invalid input (not a number or hash)', async () => {
    const mockApi = {
      rpc: { chain: { getBlock: vi.fn(), getBlockHash: vi.fn() } },
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useBlock('not-valid'), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toMatch(/Invalid block hash or number/);
    expect(result.current.data).toBeNull();
  });

  it('fetches block by hash — full data with timestamp and events', async () => {
    const mockExtrinsic = {
      hash: { toHex: () => TX_HASH },
      method: { section: 'balances', method: 'transfer' },
      isSigned: true,
      signer: { toString: () => '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY' },
    };

    const mockEvents = [
      {
        phase: {
          isApplyExtrinsic: true,
          asApplyExtrinsic: { toNumber: () => 0 },
        },
        event: { section: 'system', method: 'ExtrinsicSuccess' },
      },
    ];

    const mockApi = {
      rpc: {
        chain: {
          getBlock: vi.fn().mockResolvedValue(makeMockBlock([mockExtrinsic])),
        },
      },
      query: {
        timestamp: {
          now: {
            at: vi.fn().mockResolvedValue({ toString: () => '1700000000000' }),
          },
        },
        system: {
          events: {
            at: vi.fn().mockResolvedValue({ toHuman: () => mockEvents }),
          },
        },
        session: {
          validators: {
            at: vi.fn().mockResolvedValue({
              toJSON: () => ['5ValidatorAddr1', '5ValidatorAddr2'],
            }),
          },
        },
      },
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useBlock(BLOCK_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    const data = result.current.data;
    expect(data).not.toBeNull();
    expect(data?.hash).toBe(BLOCK_HASH);
    expect(data?.number).toBe(100);
    expect(data?.timestamp).toBe(1700000000000);
    expect(data?.parentHash).toBe(PARENT_HASH);
    expect(data?.stateRoot).toBe(STATE_ROOT);
    expect(data?.extrinsicsRoot).toBe(EXTR_ROOT);
    expect(data?.extrinsics).toHaveLength(1);
    expect(data?.extrinsics[0]?.section).toBe('balances');
    expect(data?.extrinsics[0]?.success).toBe(true);
    expect(data?.producer).toBeTruthy();
  });

  it('fetches block by number — calls getBlockHash first', async () => {
    const mockApi = {
      rpc: {
        chain: {
          getBlockHash: vi.fn().mockResolvedValue({ toHex: () => BLOCK_HASH }),
          getBlock: vi.fn().mockResolvedValue(makeMockBlock([])),
        },
      },
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useBlock('100'), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.data?.number).toBe(100);
    expect(mockApi.rpc.chain.getBlockHash).toHaveBeenCalledWith(100);
  });

  it('marks extrinsic as failed when ExtrinsicFailed event present', async () => {
    const mockExtrinsic = {
      hash: { toHex: () => TX_HASH },
      method: { section: 'system', method: 'remark' },
      isSigned: false,
      signer: null,
    };

    const failEvents = [
      {
        phase: {
          isApplyExtrinsic: true,
          asApplyExtrinsic: { toNumber: () => 0 },
        },
        event: { section: 'system', method: 'ExtrinsicFailed' },
      },
    ];

    const mockApi = {
      rpc: {
        chain: {
          getBlock: vi.fn().mockResolvedValue(makeMockBlock([mockExtrinsic])),
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
    const { result } = renderHook(() => useBlock(BLOCK_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data?.extrinsics[0]?.success).toBe(false);
    expect(result.current.data?.extrinsics[0]?.signer).toBeNull();
  });

  it('returns error when API throws', async () => {
    const mockApi = {
      rpc: {
        chain: {
          getBlock: vi.fn().mockRejectedValue(new Error('RPC failed')),
        },
      },
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useBlock(BLOCK_HASH), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBe('RPC failed');
    expect(result.current.data).toBeNull();
  });
});
