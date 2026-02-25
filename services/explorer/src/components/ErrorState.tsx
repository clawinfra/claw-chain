'use client';

import { useRouter } from 'next/navigation';

interface ErrorStateProps {
  message: string;
  backLink?: boolean;
}

export function ErrorState({ message, backLink = true }: ErrorStateProps) {
  const router = useRouter();
  return (
    <div className="flex flex-col items-center justify-center py-20 gap-4" role="alert">
      <div className="text-red-400 text-4xl">⚠</div>
      <p className="text-[#E5E7EB] text-lg font-medium">Something went wrong</p>
      <p className="text-[#9CA3AF] text-sm max-w-md text-center">{message}</p>
      {backLink && (
        <button
          onClick={() => router.back()}
          className="mt-2 px-4 py-2 rounded border border-[#1F2937] text-[#9CA3AF] hover:border-[#00D4FF] hover:text-[#00D4FF] transition-colors text-sm"
        >
          ← Go back
        </button>
      )}
    </div>
  );
}
