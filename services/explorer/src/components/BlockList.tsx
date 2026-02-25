'use client';

import Link from 'next/link';
import type { BlockSummary } from '@/lib/types';
import { shortenHash, formatTimestamp, formatAddress } from '@/lib/format';
import { LoadingState } from './LoadingState';
import { ErrorState } from './ErrorState';

interface BlockListProps {
  blocks: BlockSummary[];
  loading: boolean;
  error: string | null;
}

export function BlockList({ blocks, loading, error }: BlockListProps) {
  if (loading && blocks.length === 0) {
    return <LoadingState message="Waiting for blocks..." />;
  }

  if (error) {
    return <ErrorState message={error} backLink={false} />;
  }

  if (blocks.length === 0) {
    return (
      <div className="text-center py-20 text-[#9CA3AF]">No blocks found</div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm" aria-label="Block list">
        <thead>
          <tr className="text-left border-b border-[#1F2937]">
            <th className="py-3 px-4 text-[#9CA3AF] font-medium">Block</th>
            <th className="py-3 px-4 text-[#9CA3AF] font-medium hidden sm:table-cell">Age</th>
            <th className="py-3 px-4 text-[#9CA3AF] font-medium">Txns</th>
            <th className="py-3 px-4 text-[#9CA3AF] font-medium hidden md:table-cell">Hash</th>
            <th className="py-3 px-4 text-[#9CA3AF] font-medium hidden lg:table-cell">Producer</th>
          </tr>
        </thead>
        <tbody>
          {blocks.map((block) => (
            <tr
              key={block.hash}
              className="border-b border-[#1F2937] hover:bg-[#111111] transition-colors"
            >
              <td className="py-3 px-4">
                <Link
                  href={`/blocks/${block.hash}`}
                  className="text-[#00D4FF] hover:underline font-mono"
                >
                  #{block.number.toLocaleString()}
                </Link>
              </td>
              <td className="py-3 px-4 text-[#9CA3AF] hidden sm:table-cell">
                {block.timestamp > 0 ? formatTimestamp(block.timestamp) : '—'}
              </td>
              <td className="py-3 px-4 text-[#E5E7EB]">{block.extrinsicCount}</td>
              <td className="py-3 px-4 hidden md:table-cell">
                <Link
                  href={`/blocks/${block.hash}`}
                  className="text-[#9CA3AF] hover:text-[#00D4FF] font-mono transition-colors"
                >
                  {shortenHash(block.hash)}
                </Link>
              </td>
              <td className="py-3 px-4 hidden lg:table-cell">
                {block.producer ? (
                  <Link
                    href={`/agents/${block.producer}`}
                    className="text-[#9CA3AF] hover:text-[#00D4FF] font-mono transition-colors"
                  >
                    {formatAddress(block.producer)}
                  </Link>
                ) : (
                  <span className="text-[#4B5563]">Unknown</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
