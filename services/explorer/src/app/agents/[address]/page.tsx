'use client';

import { use } from 'react';
import Link from 'next/link';
import { useAgent } from '@/hooks/useAgent';
import { AgentProfile } from '@/components/AgentProfile';

export default function AgentProfilePage({ params }: { params: Promise<{ address: string }> }) {
  const { address } = use(params);
  const { data, loading, error } = useAgent(address);

  return (
    <div>
      <div className="mb-6">
        <Link href="/blocks" className="text-[#9CA3AF] hover:text-[#00D4FF] text-sm transition-colors">
          ← Back to Blocks
        </Link>
        <h1 className="text-2xl font-bold text-[#E5E7EB] mt-2">Agent Profile</h1>
      </div>

      <AgentProfile data={data} loading={loading} error={error} />
    </div>
  );
}
