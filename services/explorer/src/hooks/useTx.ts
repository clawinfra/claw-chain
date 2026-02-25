'use client';

import { useEffect, useState } from 'react';
import type { ExtrinsicInfo, DecodedEvent } from '@/lib/types';
import { useApi } from './useApi';

interface UseTxResult {
  data: ExtrinsicInfo | null;
  loading: boolean;
  error: string | null;
}

/**
 * Fetch extrinsic details by tx hash.
 * Note: Substrate doesn't index txs natively — we search recent blocks.
 */
export function useTx(txHash: string): UseTxResult {
  const { api } = useApi();
  const [data, setData] = useState<ExtrinsicInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!api || !txHash) return;

    let mounted = true;
    setLoading(true);
    setError(null);
    setData(null);

    if (!/^0x[0-9a-fA-F]{64}$/.test(txHash)) {
      setError(`Invalid transaction hash: ${txHash}`);
      setLoading(false);
      return;
    }

    async function findTx() {
      try {
        // Search last 256 blocks for the tx
        const latestHeader = await api!.rpc.chain.getHeader();
        const latestNum = latestHeader.number.toNumber();
        const searchDepth = Math.min(256, latestNum);

        for (let i = latestNum; i > latestNum - searchDepth; i--) {
          if (!mounted) return;

          const blockHash = await api!.rpc.chain.getBlockHash(i);
          const signedBlock = await api!.rpc.chain.getBlock(blockHash);

          const exts = signedBlock.block.extrinsics;
          const idx = exts.findIndex((ext) => ext.hash.toHex() === txHash);

          if (idx === -1) continue;

          const ext = exts[idx];
          if (!ext) continue;

          const section = ext.method.section;
          const method = ext.method.method;
          const signer = ext.isSigned ? ext.signer.toString() : null;

          // Decode args
          const args: Record<string, unknown> = {};
          try {
            const humanArgs = ext.method.args.map((a) => a.toHuman());
            ext.method.argsDef &&
              Object.keys(ext.method.argsDef).forEach((key, ki) => {
                args[key] = humanArgs[ki];
              });
          } catch {
            // args unavailable
          }

          // Fetch events for this extrinsic
          const events: DecodedEvent[] = [];
          let success = true;
          let tip = '0';
          let fee = '0';

          try {
            type EvHuman = {
              phase: { applyExtrinsic?: string };
              event: { section: string; method: string; data: Record<string, unknown> };
            };
            const systemQuery = api!.query['system'] as
              | { events: { at: (h: string) => Promise<{ toHuman(): unknown }> } }
              | undefined;
            const rawEvents = systemQuery?.events?.at
              ? await systemQuery.events.at(blockHash.toHex())
              : null;
            const evArr: EvHuman[] = rawEvents ? (rawEvents.toHuman() as EvHuman[]) : [];

            for (const ev of evArr) {
              if (ev.phase.applyExtrinsic === idx.toString()) {
                events.push({
                  section: ev.event.section,
                  method: ev.event.method,
                  data: ev.event.data,
                });
                if (ev.event.section === 'system' && ev.event.method === 'ExtrinsicFailed') {
                  success = false;
                }
                if (ev.event.section === 'transactionPayment' && ev.event.method === 'TransactionFeePaid') {
                  const d = ev.event.data as { actualFee?: string; tip?: string };
                  fee = d.actualFee ?? '0';
                  tip = d.tip ?? '0';
                }
              }
            }
          } catch {
            // events unavailable
          }

          const txInfo: ExtrinsicInfo = {
            hash: txHash,
            index: idx,
            section,
            method,
            signer,
            success,
            blockHash: blockHash.toHex(),
            blockNumber: i,
            args,
            events,
            tip,
            fee,
          };

          if (mounted) {
            setData(txInfo);
            setLoading(false);
          }
          return;
        }

        if (mounted) {
          setError('Transaction not found in recent blocks');
          setLoading(false);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : 'Failed to fetch transaction');
          setLoading(false);
        }
      }
    }

    findTx();
    return () => { mounted = false; };
  }, [api, txHash]);

  return { data, loading, error };
}
