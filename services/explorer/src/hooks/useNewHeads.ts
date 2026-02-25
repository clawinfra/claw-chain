'use client';

import { useEffect, useState } from 'react';
import type { BlockSummary } from '@/lib/types';
import type { ApiPromise } from '@polkadot/api';
import { useApi } from './useApi';

async function getTimestampAt(api: ApiPromise, blockHash: string): Promise<{ toString(): string } | null> {
  try {
    const tsQuery = api.query['timestamp'] as
      | { now: { at: (h: string) => Promise<{ toString(): string }> } }
      | undefined;
    if (!tsQuery?.now?.at) return null;
    return await tsQuery.now.at(blockHash);
  } catch {
    return null;
  }
}

const MAX_BLOCKS = 50;

interface UseNewHeadsResult {
  blocks: BlockSummary[];
  loading: boolean;
  error: string | null;
}

/**
 * Subscribe to new block heads and maintain a rolling window of the last 50 blocks.
 */
export function useNewHeads(): UseNewHeadsResult {
  const { api } = useApi();
  const [blocks, setBlocks] = useState<BlockSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!api) return;

    let unsub: (() => void) | null = null;
    let mounted = true;

    async function subscribe() {
      try {
        const unsubFn = await api!.rpc.chain.subscribeNewHeads(async (header) => {
          if (!mounted) return;

          try {
            const blockHash = header.hash;
            const [signedBlock, blockTimestamp] = await Promise.all([
              api!.rpc.chain.getBlock(blockHash),
              getTimestampAt(api!, blockHash.toHex()),
            ]);

            const extrinsics = signedBlock.block.extrinsics;
            const ts = blockTimestamp ? Number(blockTimestamp.toString()) : 0;

            // Try to get the block author/producer
            let producer = '';
            try {
              const sessionQuery = api!.query['session'] as
                | { validators: { at: (h: string) => Promise<{ toJSON(): unknown }> } }
                | undefined;
              if (sessionQuery?.validators?.at) {
                const validators = await sessionQuery.validators.at(blockHash.toHex());
                const valArr = validators.toJSON() as string[];
                const slot = header.number.toNumber() % (valArr.length || 1);
                producer = valArr[slot] ?? '';
              }
            } catch {
              producer = '';
            }

            const summary: BlockSummary = {
              hash: blockHash.toHex(),
              number: header.number.toNumber(),
              timestamp: ts,
              extrinsicCount: extrinsics.length,
              producer,
            };

            if (mounted) {
              setBlocks((prev) => {
                const next = [summary, ...prev];
                return next.slice(0, MAX_BLOCKS);
              });
              setLoading(false);
            }
          } catch (err) {
            console.error('Error processing new head:', err);
          }
        });

        unsub = unsubFn as unknown as () => void;
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : 'Subscription failed');
          setLoading(false);
        }
      }
    }

    subscribe();

    return () => {
      mounted = false;
      if (unsub) unsub();
    };
  }, [api]);

  return { blocks, loading, error };
}
