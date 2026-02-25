'use client';

import Link from 'next/link';
import { useApi } from '@/hooks/useApi';
import { LiveIndicator } from './LiveIndicator';

export function Header() {
  const { status, blockNumber } = useApi();

  return (
    <header className="sticky top-0 z-50 border-b border-[#1F2937] bg-[#0a0a0a]/95 backdrop-blur-sm">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-14">
          {/* Logo */}
          <Link href="/blocks" className="flex items-center gap-2 group">
            <span className="text-[#00D4FF] font-bold text-xl tracking-tight group-hover:opacity-80 transition-opacity">
              ⛓ ClawChain
            </span>
            <span className="hidden sm:block text-[#9CA3AF] text-sm">Explorer</span>
          </Link>

          {/* Nav */}
          <nav className="flex items-center gap-6">
            <Link
              href="/blocks"
              className="text-[#9CA3AF] hover:text-[#E5E7EB] text-sm transition-colors"
            >
              Blocks
            </Link>
            <Link
              href="/agents"
              className="text-[#9CA3AF] hover:text-[#E5E7EB] text-sm transition-colors"
            >
              Agents
            </Link>
          </nav>

          {/* Status */}
          <div className="flex items-center gap-3">
            {blockNumber !== null && (
              <span className="hidden sm:block text-[#9CA3AF] text-xs font-mono">
                #{blockNumber.toLocaleString()}
              </span>
            )}
            <LiveIndicator status={status} />
          </div>
        </div>
      </div>
    </header>
  );
}
