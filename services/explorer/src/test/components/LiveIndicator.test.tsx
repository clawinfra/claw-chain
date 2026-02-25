import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LiveIndicator } from '@/components/LiveIndicator';
import type { ConnectionStatus } from '@/lib/types';

describe('LiveIndicator', () => {
  it('shows "Live" when connected', () => {
    render(<LiveIndicator status="connected" />);
    expect(screen.getByText('Live')).toBeInTheDocument();
  });

  it('shows "Connecting" when connecting', () => {
    render(<LiveIndicator status="connecting" />);
    expect(screen.getByText('Connecting')).toBeInTheDocument();
  });

  it('shows "Reconnecting" when disconnected', () => {
    render(<LiveIndicator status="disconnected" />);
    expect(screen.getByText('Reconnecting')).toBeInTheDocument();
  });

  it('shows "Error" when error', () => {
    render(<LiveIndicator status="error" />);
    expect(screen.getByText('Error')).toBeInTheDocument();
  });

  it.each([
    ['connected', 'Live'],
    ['connecting', 'Connecting'],
    ['disconnected', 'Reconnecting'],
    ['error', 'Error'],
  ] as [ConnectionStatus, string][])(
    'renders correct label for status %s',
    (status, expectedLabel) => {
      render(<LiveIndicator status={status} />);
      expect(screen.getByText(expectedLabel)).toBeInTheDocument();
    }
  );

  it('has aria-label describing status', () => {
    render(<LiveIndicator status="connected" />);
    const el = screen.getByLabelText(/Connection status: Live/i);
    expect(el).toBeInTheDocument();
  });
});
