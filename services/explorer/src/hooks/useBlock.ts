'use client';

import { useEffect, useState } from 'react';
import type { BlockInfo, ExtrinsicSummary } from '@/lib/types';
import type { ApiPromise } from '@polkadot/api';
import { useApi } from './useApi';

/** Fetch timestamp for a given block hash, returning null if unavailable */
async function getTimestampAt(
  api: ApiPromise,
  blockHash: string,
): Promise<{ toString(): string } | null> {
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

interface UseBlockResult {
  data: BlockInfo | null;
  loading: boolean;
  error: string | null;
}

/**
 * Fetch full block details by hash or block number.
 */
export function useBlock(hashOrNumber: string): UseBlockResult {
  const { api } = useApi();
  const [data, setData] = useState<BlockInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!api || !hashOrNumber) return;

    let mounted = true;
    setLoading(true);
    setError(null);
    setData(null);

    async function fetchBlock() {
      try {
        let blockHash: string;

        // Determine if it's a number or hash
        if (/^\d+$/.test(hashOrNumber)) {
          const hash = await api!.rpc.chain.getBlockHash(parseInt(hashOrNumber, 10));
          blockHash = hash.toHex();
        } else if (/^0x[0-9a-fA-F]{64}$/.test(hashOrNumber)) {
          blockHash = hashOrNumber;
        } else {
          throw new Error(`Invalid block hash or number: ${hashOrNumber}`);
        }

        const signedBlock = await api!.rpc.chain.getBlock(blockHash);
        const blockTimestamp: { toString(): string } | null = await getTimestampAt(api!, blockHash);

        const header = signedBlock.block.header;
        const ts = blockTimestamp ? Number(blockTimestamp.toString()) : 0;

        // Fetch events for extrinsic success/failure
        type RawEvent = { phase: { isApplyExtrinsic: boolean; asApplyExtrinsic: { toNumber: () => number } }; event: { section: string; method: string } };
        let events: RawEvent[] = [];
        try {
          const systemQuery = api!.query['system'] as
            | { events: { at: (h: string) => Promise<{ toHuman(): unknown }> } }
            | undefined;
          if (systemQuery?.events?.at) {
            const rawEvents = await systemQuery.events.at(blockHash);
            events = rawEvents.toHuman() as RawEvent[];
          }
        } catch {
          events = [];
        }

        const extrinsics: ExtrinsicSummary[] = signedBlock.block.extrinsics.map((ext, index) => {
          const hash = ext.hash.toHex();
          const section = ext.method.section;
          const method = ext.method.method;
          const signer = ext.isSigned ? ext.signer.toString() : null;

          // Determine success from system.ExtrinsicSuccess/Failed events
          let success = true;
          for (const ev of events) {
            if (
              ev.phase.isApplyExtrinsic &&
              ev.phase.asApplyExtrinsic.toNumber() === index
            ) {
              if (ev.event.section === 'system' && ev.event.method === 'ExtrinsicFailed') {
                success = false;
              }
            }
          }

          return { hash, index, section, method, signer, success };
        });

        // Try to get block producer
        let producer = '';
        try {
          const sessionQuery = api!.query['session'] as
            | { validators: { at: (h: string) => Promise<{ toJSON(): unknown }> } }
            | undefined;
          if (sessionQuery?.validators?.at) {
            const validators = await sessionQuery.validators.at(blockHash);
            const valArr = validators.toJSON() as string[];
            const num = header.number.toNumber();
            const slot = num % (valArr.length || 1);
            producer = valArr[slot] ?? '';
          }
        } catch {
          producer = '';
        }

        const blockInfo: BlockInfo = {
          hash: blockHash,
          number: header.number.toNumber(),
          timestamp: ts,
          extrinsicCount: extrinsics.length,
          producer,
          parentHash: header.parentHash.toHex(),
          stateRoot: header.stateRoot.toHex(),
          extrinsicsRoot: header.extrinsicsRoot.toHex(),
          extrinsics,
        };

        if (mounted) {
          setData(blockInfo);
          setLoading(false);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : 'Failed to fetch block');
          setLoading(false);
        }
      }
    }

    fetchBlock();
    return () => { mounted = false; };
  }, [api, hashOrNumber]);

  return { data, loading, error };
}
