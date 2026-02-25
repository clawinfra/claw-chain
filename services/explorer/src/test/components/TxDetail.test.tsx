import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TxDetail } from '@/components/TxDetail';
import type { ExtrinsicInfo } from '@/lib/types';

vi.mock('next/link', () => ({
  default: ({ href, children, className }: { href: string; children: React.ReactNode; className?: string }) => (
    <a href={href} className={className}>{children}</a>
  ),
}));

vi.mock('next/navigation', () => ({
  useRouter: () => ({ back: vi.fn() }),
}));

const BLOCK_HASH = '0x' + 'b'.repeat(64);
const TX_HASH = '0x' + 'a'.repeat(64);
const SIGNER = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

const mockTx: ExtrinsicInfo = {
  hash: TX_HASH,
  index: 0,
  section: 'balances',
  method: 'transfer',
  signer: SIGNER,
  success: true,
  blockHash: BLOCK_HASH,
  blockNumber: 100,
  args: { dest: '5Dest...', value: '1000' },
  events: [
    { section: 'system', method: 'ExtrinsicSuccess', data: { weight: '100' } },
  ],
  tip: '0',
  fee: '1000000000000',
};

describe('TxDetail', () => {
  it('shows loading state', () => {
    render(<TxDetail data={null} loading={true} error={null} />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows error state', () => {
    render(<TxDetail data={null} loading={false} error="Transaction not found in recent blocks" />);
    expect(screen.getByText('Transaction not found in recent blocks')).toBeInTheDocument();
  });

  it('shows "Transaction not found" when data is null', () => {
    render(<TxDetail data={null} loading={false} error={null} />);
    expect(screen.getByText('Transaction not found')).toBeInTheDocument();
  });

  it('renders tx hash', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    expect(screen.getByText(TX_HASH)).toBeInTheDocument();
  });

  it('renders success badge for successful tx', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    expect(screen.getByText('✓ Success')).toBeInTheDocument();
  });

  it('renders failed badge for failed tx', () => {
    const failedTx: ExtrinsicInfo = { ...mockTx, success: false };
    render(<TxDetail data={failedTx} loading={false} error={null} />);
    expect(screen.getByText('✗ Failed')).toBeInTheDocument();
  });

  it('renders call as section.method', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    expect(screen.getByText('balances.transfer')).toBeInTheDocument();
  });

  it('renders block number as link', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    const blockLink = screen.getByRole('link', { name: /#100/ });
    expect(blockLink).toHaveAttribute('href', `/blocks/${BLOCK_HASH}`);
  });

  it('renders signer as link to agent profile', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    const signerLink = screen.getByRole('link', { name: /5Grwva/ });
    expect(signerLink).toHaveAttribute('href', `/agents/${SIGNER}`);
  });

  it('renders "Unsigned" when signer is null', () => {
    const unsignedTx: ExtrinsicInfo = { ...mockTx, signer: null };
    render(<TxDetail data={unsignedTx} loading={false} error={null} />);
    expect(screen.getByText('Unsigned')).toBeInTheDocument();
  });

  it('renders arguments as JSON', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    expect(screen.getByText('Arguments')).toBeInTheDocument();
    // JSON.stringify output of args
    const pre = screen.getByText(/dest/);
    expect(pre).toBeInTheDocument();
  });

  it('renders events section', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    expect(screen.getByText('Events (1)')).toBeInTheDocument();
    expect(screen.getByText('system.ExtrinsicSuccess')).toBeInTheDocument();
  });

  it('does not render Arguments section when args is empty', () => {
    const noArgsTx: ExtrinsicInfo = { ...mockTx, args: {} };
    render(<TxDetail data={noArgsTx} loading={false} error={null} />);
    expect(screen.queryByText('Arguments')).toBeNull();
  });

  it('does not render Events section when events is empty', () => {
    const noEventsTx: ExtrinsicInfo = { ...mockTx, events: [] };
    render(<TxDetail data={noEventsTx} loading={false} error={null} />);
    expect(screen.queryByText(/Events \(/)).toBeNull();
  });

  it('renders fee and tip formatted as CLAW', () => {
    render(<TxDetail data={mockTx} loading={false} error={null} />);
    // Both fee and tip rows show CLAW amounts — both display as 0.0000 CLAW
    const clawElements = screen.getAllByText(/CLAW/);
    expect(clawElements.length).toBeGreaterThanOrEqual(2); // Fee row + Tip row
  });

  it('renders multiple events', () => {
    const multiEventTx: ExtrinsicInfo = {
      ...mockTx,
      events: [
        { section: 'system', method: 'ExtrinsicSuccess', data: {} },
        { section: 'balances', method: 'Transfer', data: { from: '5A', to: '5B' } },
      ],
    };
    render(<TxDetail data={multiEventTx} loading={false} error={null} />);
    expect(screen.getByText('Events (2)')).toBeInTheDocument();
    expect(screen.getByText('system.ExtrinsicSuccess')).toBeInTheDocument();
    expect(screen.getByText('balances.Transfer')).toBeInTheDocument();
  });
});
