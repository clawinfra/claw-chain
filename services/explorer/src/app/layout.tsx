import type { Metadata } from 'next';
import './globals.css';
import { ApiProvider } from '@/providers/ApiProvider';
import { Header } from '@/components/Header';

export const metadata: Metadata = {
  title: 'ClawChain Explorer',
  description: 'Block explorer for the ClawChain agent-native L1 blockchain',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-[#0a0a0a] text-[#E5E7EB] min-h-screen">
        <ApiProvider>
          <Header />
          <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
            {children}
          </main>
        </ApiProvider>
      </body>
    </html>
  );
}
