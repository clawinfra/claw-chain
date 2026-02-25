'use client';

import { use } from 'react';
import Link from 'next/link';
import { useTx } from '@/hooks/useTx';
import { TxDetail } from '@/components/TxDetail';

export default function TxDetailPage({ params }: { params: Promise<{ hash: string }> }) {
  const { hash } = use(params);
  const { data, loading, error } = useTx(hash);

  return (
    <div>
      <div className="mb-6">
        <Link href="/blocks" className="text-[#9CA3AF] hover:text-[#00D4FF] text-sm transition-colors">
          ← Back to Blocks
        </Link>
        <h1 className="text-2xl font-bold text-[#E5E7EB] mt-2">Transaction Detail</h1>
      </div>

      <TxDetail data={data} loading={loading} error={error} />
    </div>
  );
}
