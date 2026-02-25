'use client';

import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import { ApiPromise, WsProvider } from '@polkadot/api';
import type { ConnectionStatus } from '@/lib/types';

interface ApiContextValue {
  api: ApiPromise | null;
  status: ConnectionStatus;
  blockNumber: number | null;
}

export const ApiContext = createContext<ApiContextValue>({
  api: null,
  status: 'connecting',
  blockNumber: null,
});

export function useApiContext(): ApiContextValue {
  return useContext(ApiContext);
}

const WS_URL =
  typeof window !== 'undefined'
    ? (process.env['NEXT_PUBLIC_WS_URL'] ?? 'wss://testnet.clawchain.win')
    : 'wss://testnet.clawchain.win';

const BACKOFF_MS = [1000, 2000, 4000, 8000, 16000, 30000];

function getBackoff(attempt: number): number {
  const idx = Math.min(attempt, BACKOFF_MS.length - 1);
  return BACKOFF_MS[idx] ?? 30000;
}

export function ApiProvider({ children }: { children: React.ReactNode }) {
  const [api, setApi] = useState<ApiPromise | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>('connecting');
  const [blockNumber, setBlockNumber] = useState<number | null>(null);
  const attemptRef = useRef(0);
  const unsubRef = useRef<(() => void) | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    connect();

    return () => {
      mountedRef.current = false;
      cleanup();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function cleanup() {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    if (unsubRef.current) {
      unsubRef.current();
      unsubRef.current = null;
    }
  }

  async function connect() {
    if (!mountedRef.current) return;
    setStatus('connecting');

    try {
      const provider = new WsProvider(WS_URL, false);

      provider.on('connected', () => {
        if (mountedRef.current) setStatus('connected');
      });

      provider.on('disconnected', () => {
        if (!mountedRef.current) return;
        setStatus('disconnected');
        scheduleReconnect();
      });

      provider.on('error', () => {
        if (!mountedRef.current) return;
        setStatus('error');
      });

      await provider.connect();

      const newApi = await ApiPromise.create({ provider });
      if (!mountedRef.current) {
        await newApi.disconnect();
        return;
      }

      setApi(newApi);
      setStatus('connected');
      attemptRef.current = 0;

      // Subscribe to new heads for live block number
      const unsub = await newApi.rpc.chain.subscribeNewHeads((header) => {
        if (mountedRef.current) {
          setBlockNumber(header.number.toNumber());
        }
      });

      unsubRef.current = unsub as unknown as () => void;
    } catch (err) {
      console.error('ApiProvider connect error:', err);
      if (mountedRef.current) {
        setStatus('error');
        scheduleReconnect();
      }
    }
  }

  function scheduleReconnect() {
    if (!mountedRef.current) return;
    cleanup();
    const delay = getBackoff(attemptRef.current);
    attemptRef.current += 1;
    reconnectTimerRef.current = setTimeout(() => {
      if (mountedRef.current) connect();
    }, delay);
  }

  return (
    <ApiContext.Provider value={{ api, status, blockNumber }}>
      {children}
    </ApiContext.Provider>
  );
}
