import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { AgentProfile } from '@/components/AgentProfile';
import type { AgentInfo } from '@/lib/types';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ back: vi.fn() }),
}));

const fullAgent: AgentInfo = {
  address: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  did: 'did:claw:5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  reputation: 85,
  reputationHistory: [
    { block: 100, score: 80 },
    { block: 200, score: 85 },
  ],
  gasQuota: {
    remaining: '500000000000000000',
    total: '1000000000000000000',
    lastRefill: 150,
  },
};

const nullFieldsAgent: AgentInfo = {
  address: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  did: null,
  reputation: null,
  reputationHistory: [],
  gasQuota: null,
};

describe('AgentProfile', () => {
  it('shows loading state', () => {
    render(<AgentProfile data={null} loading={true} error={null} />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows error state', () => {
    render(<AgentProfile data={null} loading={false} error="Agent not found" />);
    expect(screen.getByText('Agent not found')).toBeInTheDocument();
  });

  it('shows "Agent not found" when no data', () => {
    render(<AgentProfile data={null} loading={false} error={null} />);
    expect(screen.getByText('Agent not found')).toBeInTheDocument();
  });

  it('renders address', () => {
    render(<AgentProfile data={fullAgent} loading={false} error={null} />);
    expect(screen.getByText(fullAgent.address)).toBeInTheDocument();
  });

  it('renders DID when present', () => {
    render(<AgentProfile data={fullAgent} loading={false} error={null} />);
    expect(screen.getByText(fullAgent.did!)).toBeInTheDocument();
  });

  it('renders Unavailable badge for null DID', () => {
    render(<AgentProfile data={nullFieldsAgent} loading={false} error={null} />);
    const badges = screen.getAllByText('Unavailable');
    expect(badges.length).toBeGreaterThan(0);
  });

  it('renders reputation score', () => {
    render(<AgentProfile data={fullAgent} loading={false} error={null} />);
    // '85' appears as score + in history table — use getAllByText
    const items = screen.getAllByText('85');
    expect(items.length).toBeGreaterThan(0);
  });

  it('renders Unavailable for null reputation', () => {
    render(<AgentProfile data={nullFieldsAgent} loading={false} error={null} />);
    const badges = screen.getAllByText('Unavailable');
    expect(badges.length).toBeGreaterThan(0);
  });

  it('renders reputation history table', () => {
    render(<AgentProfile data={fullAgent} loading={false} error={null} />);
    expect(screen.getByRole('table', { name: /reputation history/i })).toBeInTheDocument();
    expect(screen.getByText('#100')).toBeInTheDocument();
    expect(screen.getByText('#200')).toBeInTheDocument();
  });

  it('renders gas quota unavailable message for null quota', () => {
    render(<AgentProfile data={nullFieldsAgent} loading={false} error={null} />);
    expect(screen.getByText(/pallet-gas-quota not available/i)).toBeInTheDocument();
  });

  it('renders gas quota values when present', () => {
    render(<AgentProfile data={fullAgent} loading={false} error={null} />);
    expect(screen.getByText(/Last Refill/i)).toBeInTheDocument();
    expect(screen.getByText(/#150/)).toBeInTheDocument();
  });
});
