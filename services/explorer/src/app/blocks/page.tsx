'use client';

import { useNewHeads } from '@/hooks/useNewHeads';
import { BlockList } from '@/components/BlockList';

export default function BlockListPage() {
  const { blocks, loading, error } = useNewHeads();

  return (
    <div>
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-[#E5E7EB]">Latest Blocks</h1>
        <p className="text-[#9CA3AF] text-sm mt-1">
          Showing last {blocks.length > 0 ? blocks.length : '—'} blocks
        </p>
      </div>

      <div className="bg-[#111111] rounded-lg border border-[#1F2937]">
        <BlockList blocks={blocks} loading={loading} error={error} />
      </div>
    </div>
  );
}
