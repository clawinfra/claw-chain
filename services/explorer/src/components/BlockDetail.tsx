'use client';

import Link from 'next/link';
import type { BlockInfo } from '@/lib/types';
import { shortenHash, formatTimestamp, formatAddress } from '@/lib/format';
import { LoadingState } from './LoadingState';
import { ErrorState } from './ErrorState';

interface BlockDetailProps {
  data: BlockInfo | null;
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

export function BlockDetail({ data, loading, error }: BlockDetailProps) {
  if (loading) return <LoadingState message="Loading block..." />;
  if (error) return <ErrorState message={error} />;
  if (!data) return <ErrorState message="Block not found" />;

  return (
    <div className="space-y-6">
      <div className="bg-[#111111] rounded-lg border border-[#1F2937] p-6">
        <h2 className="text-[#E5E7EB] font-semibold mb-4">
          Block{' '}
          <span className="text-[#00D4FF] font-mono">#{data.number.toLocaleString()}</span>
        </h2>
        <dl>
          <InfoRow label="Hash" value={data.hash} />
          <InfoRow label="Parent Hash" value={
            <Link href={`/blocks/${data.parentHash}`} className="text-[#00D4FF] hover:underline">
              {shortenHash(data.parentHash)}
            </Link>
          } />
          <InfoRow
            label="Timestamp"
            value={data.timestamp > 0 ? `${formatTimestamp(data.timestamp)} (${formatTimestamp(data.timestamp, false)})` : 'Unknown'}
          />
          <InfoRow label="State Root" value={shortenHash(data.stateRoot)} />
          <InfoRow label="Extrinsics Root" value={shortenHash(data.extrinsicsRoot)} />
          <InfoRow label="Extrinsics" value={data.extrinsicCount} />
          <InfoRow
            label="Producer"
            value={
              data.producer ? (
                <Link href={`/agents/${data.producer}`} className="text-[#00D4FF] hover:underline">
                  {formatAddress(data.producer)}
                </Link>
              ) : (
                <span className="text-[#4B5563]">Unknown</span>
              )
            }
          />
        </dl>
      </div>

      {/* Extrinsics table */}
      {data.extrinsics.length > 0 && (
        <div className="bg-[#111111] rounded-lg border border-[#1F2937]">
          <div className="px-6 py-4 border-b border-[#1F2937]">
            <h3 className="text-[#E5E7EB] font-semibold">
              Extrinsics ({data.extrinsics.length})
            </h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm" aria-label="Extrinsics list">
              <thead>
                <tr className="border-b border-[#1F2937]">
                  <th className="py-3 px-4 text-left text-[#9CA3AF] font-medium">#</th>
                  <th className="py-3 px-4 text-left text-[#9CA3AF] font-medium">Hash</th>
                  <th className="py-3 px-4 text-left text-[#9CA3AF] font-medium">Call</th>
                  <th className="py-3 px-4 text-left text-[#9CA3AF] font-medium hidden md:table-cell">Signer</th>
                  <th className="py-3 px-4 text-left text-[#9CA3AF] font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                {data.extrinsics.map((ext) => (
                  <tr
                    key={ext.hash}
                    className="border-b border-[#1F2937] hover:bg-[#0a0a0a] transition-colors"
                  >
                    <td className="py-3 px-4 text-[#9CA3AF] font-mono">{ext.index}</td>
                    <td className="py-3 px-4">
                      <Link
                        href={`/tx/${ext.hash}`}
                        className="text-[#00D4FF] hover:underline font-mono"
                      >
                        {shortenHash(ext.hash)}
                      </Link>
                    </td>
                    <td className="py-3 px-4 text-[#E5E7EB]">
                      {ext.section}.{ext.method}
                    </td>
                    <td className="py-3 px-4 hidden md:table-cell">
                      {ext.signer ? (
                        <Link
                          href={`/agents/${ext.signer}`}
                          className="text-[#9CA3AF] hover:text-[#00D4FF] font-mono transition-colors"
                        >
                          {formatAddress(ext.signer)}
                        </Link>
                      ) : (
                        <span className="text-[#4B5563]">—</span>
                      )}
                    </td>
                    <td className="py-3 px-4">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                          ext.success
                            ? 'bg-green-900/30 text-green-400'
                            : 'bg-red-900/30 text-red-400'
                        }`}
                      >
                        {ext.success ? '✓ Success' : '✗ Failed'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
