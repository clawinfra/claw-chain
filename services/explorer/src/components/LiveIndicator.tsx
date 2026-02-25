'use client';

import type { ConnectionStatus } from '@/lib/types';

interface LiveIndicatorProps {
  status: ConnectionStatus;
}

const STATUS_CONFIG: Record<ConnectionStatus, { color: string; label: string; pulse: boolean }> = {
  connected: { color: 'bg-green-400', label: 'Live', pulse: true },
  connecting: { color: 'bg-yellow-400', label: 'Connecting', pulse: true },
  disconnected: { color: 'bg-yellow-400', label: 'Reconnecting', pulse: true },
  error: { color: 'bg-red-500', label: 'Error', pulse: false },
};

export function LiveIndicator({ status }: LiveIndicatorProps) {
  const cfg = STATUS_CONFIG[status];
  return (
    <div className="flex items-center gap-1.5" aria-label={`Connection status: ${cfg.label}`}>
      <div className="relative flex h-2 w-2">
        {cfg.pulse && (
          <span
            className={`animate-ping absolute inline-flex h-full w-full rounded-full ${cfg.color} opacity-75`}
          />
        )}
        <span className={`relative inline-flex rounded-full h-2 w-2 ${cfg.color}`} />
      </div>
      <span className="text-xs text-[#9CA3AF]">{cfg.label}</span>
    </div>
  );
}
