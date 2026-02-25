import { describe, it, expect, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import React from 'react';
import { useApi } from '@/hooks/useApi';
import { ApiContext } from '@/providers/ApiProvider';
import type { ConnectionStatus } from '@/lib/types';

// We test useApi through the context
describe('useApi', () => {
  it('returns the context value', () => {
    const contextValue = {
      api: null,
      status: 'connected' as ConnectionStatus,
      blockNumber: 42,
    };

    const wrapper = ({ children }: { children: React.ReactNode }) =>
      React.createElement(ApiContext.Provider, { value: contextValue }, children);

    const { result } = renderHook(() => useApi(), { wrapper });
    expect(result.current.status).toBe('connected');
    expect(result.current.blockNumber).toBe(42);
    expect(result.current.api).toBeNull();
  });

  it('returns default connecting status when no provider', () => {
    const { result } = renderHook(() => useApi());
    expect(result.current.status).toBe('connecting');
    expect(result.current.blockNumber).toBeNull();
  });
});
