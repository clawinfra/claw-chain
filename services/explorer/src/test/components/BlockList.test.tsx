import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BlockList } from '@/components/BlockList';
import type { BlockSummary } from '@/lib/types';

// Mock next/navigation
vi.mock('next/navigation', () => ({
  useRouter: () => ({ back: vi.fn() }),
}));

// Mock next/link
vi.mock('next/link', () => ({
  default: ({ children, href }: { children: React.ReactNode; href: string }) => (
    <a href={href}>{children}</a>
  ),
}));

const mockBlocks: BlockSummary[] = [
  {
    hash: '0x' + 'a'.repeat(64),
    number: 42,
    timestamp: Date.now() - 5000,
    extrinsicCount: 3,
    producer: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  },
  {
    hash: '0x' + 'b'.repeat(64),
    number: 41,
    timestamp: Date.now() - 11000,
    extrinsicCount: 1,
    producer: '',
  },
];

describe('BlockList', () => {
  it('shows loading state when loading with no blocks', () => {
    render(<BlockList blocks={[]} loading={true} error={null} />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows error state when error provided', () => {
    render(<BlockList blocks={[]} loading={false} error="Connection failed" />);
    expect(screen.getByText('Connection failed')).toBeInTheDocument();
  });

  it('shows empty message when no blocks', () => {
    render(<BlockList blocks={[]} loading={false} error={null} />);
    expect(screen.getByText(/No blocks found/i)).toBeInTheDocument();
  });

  it('renders block numbers', () => {
    render(<BlockList blocks={mockBlocks} loading={false} error={null} />);
    expect(screen.getByText('#42')).toBeInTheDocument();
    expect(screen.getByText('#41')).toBeInTheDocument();
  });

  it('renders extrinsic counts', () => {
    render(<BlockList blocks={mockBlocks} loading={false} error={null} />);
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('links to block detail page', () => {
    render(<BlockList blocks={mockBlocks} loading={false} error={null} />);
    const links = screen.getAllByRole('link');
    const blockLinks = links.filter((l) => l.getAttribute('href')?.startsWith('/blocks/'));
    expect(blockLinks.length).toBeGreaterThan(0);
  });

  it('shows Unknown for block with empty producer', () => {
    render(<BlockList blocks={mockBlocks} loading={false} error={null} />);
    expect(screen.getByText('Unknown')).toBeInTheDocument();
  });

  it('renders the table with aria-label', () => {
    render(<BlockList blocks={mockBlocks} loading={false} error={null} />);
    expect(screen.getByRole('table', { name: /block list/i })).toBeInTheDocument();
  });
});
