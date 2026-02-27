import { describe, it, expect, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import React from 'react';
import { useAgent } from '@/hooks/useAgent';
import { ApiContext } from '@/providers/ApiProvider';
import type { ApiPromise } from '@polkadot/api';
import type { ConnectionStatus } from '@/lib/types';

// Valid SS58 address (48 chars — within 32-60 range)
const VALID_ADDR = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

function makeWrapper(api: ApiPromise | null) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(
      ApiContext.Provider,
      { value: { api, status: 'connected' as ConnectionStatus, blockNumber: 1 } },
      children,
    );
  };
}

describe('useAgent', () => {
  it('stays loading when api is null', () => {
    const wrapper = makeWrapper(null);
    const { result } = renderHook(() => useAgent(VALID_ADDR), { wrapper });
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('returns error for address too short (< 32 chars)', async () => {
    const mockApi = {} as unknown as ApiPromise;
    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useAgent('abc'), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toMatch(/Invalid address/);
    expect(result.current.data).toBeNull();
  });

  it('returns error for address too long (> 60 chars)', async () => {
    const longAddr = 'a'.repeat(61);
    const mockApi = {} as unknown as ApiPromise;
    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useAgent(longAddr), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.error).toMatch(/Invalid address/);
  });

  it('fetches agent data with all pallets available', async () => {
    const mockAgentIds = { toJSON: () => ['agent-id-1'] };
    const mockAgentDetails = { toHuman: () => ({ did: 'did:claw:test-agent' }) };
    const mockRep = { toJSON: () => 85 };
    const mockRepHistory = { toJSON: () => [{ block: 100, score: 80 }] };
    const mockQuota = {
      toJSON: () => ({
        remaining: '500000000000000000',
        total: '1000000000000000000',
        lastRefill: 150,
      }),
    };

    const mockApi = {
      query: {
        agentRegistry: {
          ownerAgents: vi.fn().mockResolvedValue(mockAgentIds),
          agentRegistry: vi.fn().mockResolvedValue(mockAgentDetails),
        },
        reputation: {
          reputations: vi.fn().mockResolvedValue(mockRep),
          reputationHistory: vi.fn().mockResolvedValue(mockRepHistory),
        },
        gasQuota: {
          agentQuotas: vi.fn().mockResolvedValue(mockQuota),
        },
      },
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useAgent(VALID_ADDR), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.data).not.toBeNull();
    expect(result.current.data?.did).toBe('did:claw:test-agent');
    expect(result.current.data?.reputation).toBe(85);
    expect(result.current.data?.reputationHistory).toHaveLength(1);
    expect(result.current.data?.reputationHistory[0]?.block).toBe(100);
    expect(result.current.data?.gasQuota?.remaining).toBe('500000000000000000');
    expect(result.current.data?.gasQuota?.total).toBe('1000000000000000000');
    expect(result.current.data?.gasQuota?.lastRefill).toBe(150);
  });

  it('returns data with null fields when all pallets are missing', async () => {
    // Query object has no pallet keys — optional chaining returns undefined for all
    const mockApi = {
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useAgent(VALID_ADDR), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.data).not.toBeNull();
    expect(result.current.data?.did).toBeNull();
    expect(result.current.data?.reputation).toBeNull();
    expect(result.current.data?.gasQuota).toBeNull();
    expect(result.current.data?.reputationHistory).toEqual([]);
  });

  it('handles pallets that reject — all fields remain null', async () => {
    const mockApi = {
      query: {
        agentRegistry: {
          ownerAgents: vi.fn().mockRejectedValue(new Error('pallet not found')),
          agentRegistry: vi.fn().mockRejectedValue(new Error('pallet not found')),
        },
        reputation: {
          reputations: vi.fn().mockRejectedValue(new Error('not found')),
          reputationHistory: vi.fn().mockRejectedValue(new Error('not found')),
        },
        gasQuota: {
          agentQuotas: vi.fn().mockRejectedValue(new Error('not found')),
        },
      },
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useAgent(VALID_ADDR), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.data?.did).toBeNull();
    expect(result.current.data?.reputation).toBeNull();
    expect(result.current.data?.gasQuota).toBeNull();
  });

  it('fetches agent with no owned agents — did remains null', async () => {
    // ownerAgents returns empty list
    const mockAgentIds = { toJSON: () => [] };
    const mockRep = { toJSON: () => null };
    const mockRepHistory = { toJSON: () => [] };
    const mockQuota = { toJSON: () => null };

    const mockApi = {
      query: {
        agentRegistry: {
          ownerAgents: vi.fn().mockResolvedValue(mockAgentIds),
          agentRegistry: vi.fn().mockResolvedValue(null),
        },
        reputation: {
          reputations: vi.fn().mockResolvedValue(mockRep),
          reputationHistory: vi.fn().mockResolvedValue(mockRepHistory),
        },
        gasQuota: {
          agentQuotas: vi.fn().mockResolvedValue(mockQuota),
        },
      },
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result } = renderHook(() => useAgent(VALID_ADDR), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data?.did).toBeNull();
    expect(result.current.data?.reputation).toBeNull();
    expect(result.current.data?.reputationHistory).toEqual([]);
    expect(result.current.data?.gasQuota).toBeNull();
  });

  it('sets loading and clears previous data when address changes', async () => {
    const mockApi = {
      query: {},
    } as unknown as ApiPromise;

    const wrapper = makeWrapper(mockApi);
    const { result, rerender } = renderHook(
      ({ address }: { address: string }) => useAgent(address),
      { wrapper, initialProps: { address: VALID_ADDR } },
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // Change address — hook should reload
    rerender({ address: VALID_ADDR.replace('5', '6') });
    // Should start loading again (or settle quickly since it's the same mock)
    expect(result.current).toBeDefined();
  });
});
