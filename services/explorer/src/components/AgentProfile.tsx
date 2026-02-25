'use client';

import type { AgentInfo } from '@/lib/types';
import { formatAddress, formatBalance } from '@/lib/format';
import { LoadingState } from './LoadingState';
import { ErrorState } from './ErrorState';

interface AgentProfileProps {
  data: AgentInfo | null;
  loading: boolean;
  error: string | null;
}

function UnavailableBadge() {
  return (
    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-[#1F2937] text-[#9CA3AF]">
      Unavailable
    </span>
  );
}

function InfoRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-4 py-3 border-b border-[#1F2937]">
      <dt className="text-[#9CA3AF] text-sm w-44 shrink-0">{label}</dt>
      <dd className="text-[#E5E7EB] text-sm font-mono break-all">{value}</dd>
    </div>
  );
}

export function AgentProfile({ data, loading, error }: AgentProfileProps) {
  if (loading) return <LoadingState message="Loading agent profile..." />;
  if (error) return <ErrorState message={error} />;
  if (!data) return <ErrorState message="Agent not found" />;

  return (
    <div className="space-y-6">
      {/* Identity */}
      <div className="bg-[#111111] rounded-lg border border-[#1F2937] p-6">
        <h2 className="text-[#E5E7EB] font-semibold mb-4">Agent Profile</h2>
        <dl>
          <InfoRow label="Address" value={data.address} />
          <InfoRow
            label="DID"
            value={data.did ?? <UnavailableBadge />}
          />
        </dl>
      </div>

      {/* Reputation */}
      <div className="bg-[#111111] rounded-lg border border-[#1F2937] p-6">
        <h3 className="text-[#E5E7EB] font-semibold mb-4">Reputation</h3>
        <dl>
          <InfoRow
            label="Score"
            value={
              data.reputation !== null ? (
                <span className="text-[#00D4FF]">{data.reputation}</span>
              ) : (
                <UnavailableBadge />
              )
            }
          />
        </dl>

        {data.reputationHistory.length > 0 ? (
          <div className="mt-4">
            <h4 className="text-[#9CA3AF] text-xs uppercase tracking-wider mb-3">History</h4>
            <div className="overflow-x-auto">
              <table className="w-full text-sm" aria-label="Reputation history">
                <thead>
                  <tr className="border-b border-[#1F2937]">
                    <th className="py-2 px-3 text-left text-[#9CA3AF] font-medium">Block</th>
                    <th className="py-2 px-3 text-left text-[#9CA3AF] font-medium">Score</th>
                  </tr>
                </thead>
                <tbody>
                  {data.reputationHistory.map((entry, i) => (
                    <tr key={i} className="border-b border-[#1F2937]">
                      <td className="py-2 px-3 font-mono text-[#E5E7EB]">
                        #{entry.block.toLocaleString()}
                      </td>
                      <td className="py-2 px-3 font-mono text-[#00D4FF]">{entry.score}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : (
          data.reputation === null && (
            <p className="text-[#9CA3AF] text-sm mt-3">
              pallet-reputation not available on this runtime
            </p>
          )
        )}
      </div>

      {/* Gas Quota */}
      <div className="bg-[#111111] rounded-lg border border-[#1F2937] p-6">
        <h3 className="text-[#E5E7EB] font-semibold mb-4">Gas Quota</h3>
        {data.gasQuota ? (
          <dl>
            <InfoRow label="Remaining" value={formatBalance(data.gasQuota.remaining)} />
            <InfoRow label="Total" value={formatBalance(data.gasQuota.total)} />
            <InfoRow
              label="Last Refill"
              value={
                data.gasQuota.lastRefill > 0
                  ? `Block #${data.gasQuota.lastRefill.toLocaleString()}`
                  : '—'
              }
            />
          </dl>
        ) : (
          <div className="flex items-center gap-2">
            <UnavailableBadge />
            <span className="text-[#9CA3AF] text-sm">
              pallet-gas-quota not available on this runtime
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
