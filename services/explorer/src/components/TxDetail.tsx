'use client';

import Link from 'next/link';
import type { ExtrinsicInfo } from '@/lib/types';
import { shortenHash, formatAddress, formatBalance } from '@/lib/format';
import { LoadingState } from './LoadingState';
import { ErrorState } from './ErrorState';

interface TxDetailProps {
  data: ExtrinsicInfo | null;
  loading: boolean;
  error: string | null;
}

function InfoRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-4 py-3 border-b border-[#1F2937]">
      <dt className="text-[#9CA3AF] text-sm w-40 shrink-0">{label}</dt>
      <dd className="text-[#E5E7EB] text-sm font-mono break-all">{value}</dd>
    </div>
  );
}

export function TxDetail({ data, loading, error }: TxDetailProps) {
  if (loading) return <LoadingState message="Searching for transaction..." />;
  if (error) return <ErrorState message={error} />;
  if (!data) return <ErrorState message="Transaction not found" />;

  return (
    <div className="space-y-6">
      <div className="bg-[#111111] rounded-lg border border-[#1F2937] p-6">
        <div className="flex items-center gap-3 mb-4">
          <h2 className="text-[#E5E7EB] font-semibold">Transaction Detail</h2>
          <span
            className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
              data.success
                ? 'bg-green-900/30 text-green-400'
                : 'bg-red-900/30 text-red-400'
            }`}
          >
            {data.success ? '✓ Success' : '✗ Failed'}
          </span>
        </div>
        <dl>
          <InfoRow label="Hash" value={data.hash} />
          <InfoRow
            label="Block"
            value={
              <Link href={`/blocks/${data.blockHash}`} className="text-[#00D4FF] hover:underline">
                #{data.blockNumber.toLocaleString()} ({shortenHash(data.blockHash)})
              </Link>
            }
          />
          <InfoRow label="Call" value={`${data.section}.${data.method}`} />
          <InfoRow
            label="Signer"
            value={
              data.signer ? (
                <Link href={`/agents/${data.signer}`} className="text-[#00D4FF] hover:underline">
                  {formatAddress(data.signer)}
                </Link>
              ) : (
                <span className="text-[#4B5563]">Unsigned</span>
              )
            }
          />
          <InfoRow label="Fee" value={formatBalance(data.fee)} />
          <InfoRow label="Tip" value={formatBalance(data.tip)} />
        </dl>
      </div>

      {/* Args */}
      {Object.keys(data.args).length > 0 && (
        <div className="bg-[#111111] rounded-lg border border-[#1F2937] p-6">
          <h3 className="text-[#E5E7EB] font-semibold mb-4">Arguments</h3>
          <pre className="text-[#9CA3AF] text-xs overflow-x-auto bg-[#0a0a0a] rounded p-4">
            {JSON.stringify(data.args, null, 2)}
          </pre>
        </div>
      )}

      {/* Events */}
      {data.events.length > 0 && (
        <div className="bg-[#111111] rounded-lg border border-[#1F2937]">
          <div className="px-6 py-4 border-b border-[#1F2937]">
            <h3 className="text-[#E5E7EB] font-semibold">Events ({data.events.length})</h3>
          </div>
          <div className="divide-y divide-[#1F2937]">
            {data.events.map((ev, i) => (
              <div key={i} className="px-6 py-4">
                <p className="text-[#00D4FF] text-sm font-mono mb-2">
                  {ev.section}.{ev.method}
                </p>
                <pre className="text-[#9CA3AF] text-xs overflow-x-auto bg-[#0a0a0a] rounded p-3">
                  {JSON.stringify(ev.data, null, 2)}
                </pre>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
