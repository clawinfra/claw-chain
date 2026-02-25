import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Header } from '@/components/Header';
import type { ConnectionStatus } from '@/lib/types';

// Mock useApi hook
vi.mock('@/hooks/useApi', () => ({
  useApi: vi.fn(),
}));

// Mock next/link to render a plain anchor
vi.mock('next/link', () => ({
  default: ({ href, children, className }: { href: string; children: React.ReactNode; className?: string }) => (
    <a href={href} className={className}>{children}</a>
  ),
}));

// Mock LiveIndicator to avoid its internals
vi.mock('@/components/LiveIndicator', () => ({
  LiveIndicator: ({ status }: { status: ConnectionStatus }) => (
    <span data-testid="live-indicator" data-status={status}>{status}</span>
  ),
}));

import { useApi } from '@/hooks/useApi';

const mockUseApi = vi.mocked(useApi);

describe('Header', () => {
  it('renders logo and nav links', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'connecting', blockNumber: null });
    render(<Header />);

    expect(screen.getByText(/ClawChain/)).toBeInTheDocument();
    expect(screen.getByText('Explorer')).toBeInTheDocument();
    expect(screen.getByText('Blocks')).toBeInTheDocument();
    expect(screen.getByText('Agents')).toBeInTheDocument();
  });

  it('shows block number when available', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'connected', blockNumber: 12345 });
    render(<Header />);

    expect(screen.getByText(/#12,345|#12345/)).toBeInTheDocument();
  });

  it('hides block number when null', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'connected', blockNumber: null });
    render(<Header />);

    // No block number text
    expect(screen.queryByText(/#\d/)).toBeNull();
  });

  it('passes connection status to LiveIndicator', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'disconnected', blockNumber: null });
    render(<Header />);

    const indicator = screen.getByTestId('live-indicator');
    expect(indicator).toHaveAttribute('data-status', 'disconnected');
  });

  it('passes error status to LiveIndicator', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'error', blockNumber: null });
    render(<Header />);

    const indicator = screen.getByTestId('live-indicator');
    expect(indicator).toHaveAttribute('data-status', 'error');
  });

  it('logo links to /blocks', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'connected', blockNumber: null });
    render(<Header />);

    const logoLink = screen.getByText(/ClawChain/).closest('a');
    expect(logoLink).toHaveAttribute('href', '/blocks');
  });

  it('Blocks nav link points to /blocks', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'connected', blockNumber: null });
    render(<Header />);

    const blocksLink = screen.getByText('Blocks').closest('a');
    expect(blocksLink).toHaveAttribute('href', '/blocks');
  });

  it('Agents nav link points to /agents', () => {
    mockUseApi.mockReturnValue({ api: null, status: 'connected', blockNumber: null });
    render(<Header />);

    const agentsLink = screen.getByText('Agents').closest('a');
    expect(agentsLink).toHaveAttribute('href', '/agents');
  });
});
