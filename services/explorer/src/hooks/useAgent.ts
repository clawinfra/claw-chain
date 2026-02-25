'use client';

import { useEffect, useState } from 'react';
import type { AgentInfo } from '@/lib/types';
import { useApi } from './useApi';

interface UseAgentResult {
  data: AgentInfo | null;
  loading: boolean;
  error: string | null;
}

/**
 * Fetch on-chain agent data for a given SS58 address.
 * Uses pallet-agent-registry, pallet-reputation, and pallet-gas-quota.
 * Each query is wrapped in try/catch — pallets may not be present in all runtime versions.
 */
export function useAgent(address: string): UseAgentResult {
  const { api } = useApi();
  const [data, setData] = useState<AgentInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!api || !address) return;

    let mounted = true;
    setLoading(true);
    setError(null);
    setData(null);

    // Basic SS58 validation (length check)
    if (address.length < 32 || address.length > 60) {
      setError(`Invalid address: ${address}`);
      setLoading(false);
      return;
    }

    async function fetchAgent() {
      const info: AgentInfo = {
        address,
        did: null,
        reputation: null,
        reputationHistory: [],
        gasQuota: null,
      };

      // --- pallet-agent-registry ---
      try {
        // Get agent IDs owned by this address
        const ownerAgentsResult = await (api!.query as Record<string, Record<string, (arg: string) => Promise<unknown>>>)['agentRegistry']?.['ownerAgents']?.(address);
        if (ownerAgentsResult) {
          const agentIds = ownerAgentsResult as unknown as { toJSON(): unknown[] };
          const ids = agentIds.toJSON() as unknown[];

          // Get details for first agent
          if (ids.length > 0) {
            const firstId = ids[0];
            const agentDetails = await (api!.query as Record<string, Record<string, (arg: unknown) => Promise<unknown>>>)['agentRegistry']?.['agentRegistry']?.(firstId);
            if (agentDetails) {
              const details = (agentDetails as { toHuman(): Record<string, unknown> }).toHuman() as Record<string, unknown>;
              info.did = (details['did'] as string) ?? null;
            }
          }
        }
      } catch {
        // pallet-agent-registry not available — leave did as null
      }

      // --- pallet-reputation ---
      try {
        const repResult = await (api!.query as Record<string, Record<string, (arg: string) => Promise<unknown>>>)['reputation']?.['reputations']?.(address);
        if (repResult) {
          const repRaw = (repResult as { toJSON(): unknown }).toJSON();
          if (repRaw !== null && repRaw !== undefined) {
            info.reputation = Number(repRaw);
          }
        }
      } catch {
        // pallet-reputation not available
      }

      try {
        const histResult = await (api!.query as Record<string, Record<string, (arg: string) => Promise<unknown>>>)['reputation']?.['reputationHistory']?.(address);
        if (histResult) {
          const hist = (histResult as { toJSON(): unknown }).toJSON() as { block: number; score: number }[] | null;
          info.reputationHistory = Array.isArray(hist) ? hist : [];
        }
      } catch {
        // history unavailable
      }

      // --- pallet-gas-quota ---
      try {
        const quotaResult = await (api!.query as Record<string, Record<string, (arg: string) => Promise<unknown>>>)['gasQuota']?.['agentQuotas']?.(address);
        if (quotaResult) {
          const quota = (quotaResult as { toJSON(): unknown }).toJSON() as {
            remaining?: string | number;
            total?: string | number;
            lastRefill?: number;
          } | null;
          if (quota && typeof quota === 'object') {
            info.gasQuota = {
              remaining: String(quota.remaining ?? '0'),
              total: String(quota.total ?? '0'),
              lastRefill: Number(quota.lastRefill ?? 0),
            };
          }
        }
      } catch {
        // pallet-gas-quota not available
      }

      if (mounted) {
        setData(info);
        setLoading(false);
      }
    }

    fetchAgent().catch((err) => {
      if (mounted) {
        setError(err instanceof Error ? err.message : 'Failed to fetch agent data');
        setLoading(false);
      }
    });

    return () => { mounted = false; };
  }, [api, address]);

  return { data, loading, error };
}
