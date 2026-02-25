'use client';

import { use } from 'react';
import Link from 'next/link';
import { useBlock } from '@/hooks/useBlock';
import { BlockDetail } from '@/components/BlockDetail';

export default function BlockDetailPage({ params }: { params: Promise<{ hash: string }> }) {
  const { hash } = use(params);
  const { data, loading, error } = useBlock(hash);

  return (
    <div>
      <div className="mb-6">
        <Link href="/blocks" className="text-[#9CA3AF] hover:text-[#00D4FF] text-sm transition-colors">
          ← Back to Blocks
        </Link>
        <h1 className="text-2xl font-bold text-[#E5E7EB] mt-2">Block Detail</h1>
      </div>

      <BlockDetail data={data} loading={loading} error={error} />
    </div>
  );
}
