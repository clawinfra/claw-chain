import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BlockDetail } from '@/components/BlockDetail';
import type { BlockInfo } from '@/lib/types';

vi.mock('next/link', () => ({
  default: ({ href, children, className }: { href: string; children: React.ReactNode; className?: string }) => (
    <a href={href} className={className}>{children}</a>
  ),
}));

vi.mock('next/navigation', () => ({
  useRouter: () => ({ back: vi.fn() }),
}));

const HASH = '0x' + 'a'.repeat(64);
const PARENT = '0x' + 'b'.repeat(64);
const STATE = '0x' + 'c'.repeat(64);
const EXTR_ROOT = '0x' + 'd'.repeat(64);
const TX_HASH = '0x' + 'e'.repeat(64);
const SIGNER = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

const mockBlock: BlockInfo = {
  hash: HASH,
  number: 100,
  timestamp: 1700000000000,
  extrinsicCount: 1,
  producer: SIGNER,
  parentHash: PARENT,
  stateRoot: STATE,
  extrinsicsRoot: EXTR_ROOT,
  extrinsics: [
    {
      hash: TX_HASH,
      index: 0,
      section: 'balances',
      method: 'transfer',
      signer: SIGNER,
      success: true,
    },
  ],
};

describe('BlockDetail', () => {
  it('shows loading state', () => {
    render(<BlockDetail data={null} loading={true} error={null} />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows error state', () => {
    render(<BlockDetail data={null} loading={false} error="Block not found on chain" />);
    expect(screen.getByText('Block not found on chain')).toBeInTheDocument();
  });

  it('shows "Block not found" when data is null and no error', () => {
    render(<BlockDetail data={null} loading={false} error={null} />);
    expect(screen.getByText('Block not found')).toBeInTheDocument();
  });

  it('renders block number in heading', () => {
    render(<BlockDetail data={mockBlock} loading={false} error={null} />);
    expect(screen.getByText('#100')).toBeInTheDocument();
  });

  it('renders block hash', () => {
    render(<BlockDetail data={mockBlock} loading={false} error={null} />);
    expect(screen.getByText(HASH)).toBeInTheDocument();
  });

  it('renders parent hash as link', () => {
    render(<BlockDetail data={mockBlock} loading={false} error={null} />);
    const parentLink = screen.getByRole('link', { name: /0xbbbb/ });
    expect(parentLink).toHaveAttribute('href', `/blocks/${PARENT}`);
  });

  it('renders extrinsics table with success badge', () => {
    render(<BlockDetail data={mockBlock} loading={false} error={null} />);
    expect(screen.getByRole('table', { name: /extrinsics/i })).toBeInTheDocument();
    expect(screen.getByText('✓ Success')).toBeInTheDocument();
    expect(screen.getByText('balances.transfer')).toBeInTheDocument();
  });

  it('renders failed extrinsic badge', () => {
    const failBlock: BlockInfo = {
      ...mockBlock,
      extrinsics: [{ ...mockBlock.extrinsics[0]!, success: false }],
    };
    render(<BlockDetail data={failBlock} loading={false} error={null} />);
    expect(screen.getByText('✗ Failed')).toBeInTheDocument();
  });

  it('renders unsigned extrinsic (no signer) with dash', () => {
    const unsignedBlock: BlockInfo = {
      ...mockBlock,
      extrinsics: [{ ...mockBlock.extrinsics[0]!, signer: null }],
    };
    render(<BlockDetail data={unsignedBlock} loading={false} error={null} />);
    expect(screen.getByText('—')).toBeInTheDocument();
  });

  it('renders "Unknown" producer when producer is empty', () => {
    const noProducerBlock: BlockInfo = { ...mockBlock, producer: '' };
    render(<BlockDetail data={noProducerBlock} loading={false} error={null} />);
    expect(screen.getByText('Unknown')).toBeInTheDocument();
  });

  it('renders producer as link to agent profile', () => {
    render(<BlockDetail data={mockBlock} loading={false} error={null} />);
    // Both producer row and signer column in extrinsics table link to the same agent
    const agentLinks = screen.getAllByRole('link', { name: /5Grwva/ });
    expect(agentLinks.length).toBeGreaterThan(0);
    agentLinks.forEach(link => {
      expect(link).toHaveAttribute('href', `/agents/${SIGNER}`);
    });
  });

  it('renders tx hash as link to tx detail', () => {
    render(<BlockDetail data={mockBlock} loading={false} error={null} />);
    const txLink = screen.getByRole('link', { name: /0xeeee/ });
    expect(txLink).toHaveAttribute('href', `/tx/${TX_HASH}`);
  });

  it('renders zero timestamp as "Unknown"', () => {
    const noTsBlock: BlockInfo = { ...mockBlock, timestamp: 0 };
    render(<BlockDetail data={noTsBlock} loading={false} error={null} />);
    expect(screen.getByText('Unknown')).toBeInTheDocument();
  });

  it('renders block with no extrinsics — no table rendered', () => {
    const emptyBlock: BlockInfo = { ...mockBlock, extrinsics: [], extrinsicCount: 0 };
    render(<BlockDetail data={emptyBlock} loading={false} error={null} />);
    expect(screen.queryByRole('table')).toBeNull();
  });
});
