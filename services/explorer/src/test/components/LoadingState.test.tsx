import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LoadingState } from '@/components/LoadingState';

describe('LoadingState', () => {
  it('renders default message', () => {
    render(<LoadingState />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('renders custom message', () => {
    render(<LoadingState message="Fetching blocks..." />);
    expect(screen.getByText('Fetching blocks...')).toBeInTheDocument();
  });

  it('has role status', () => {
    render(<LoadingState />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('has aria-label matching message', () => {
    render(<LoadingState message="Custom label" />);
    const el = screen.getByRole('status');
    expect(el).toHaveAttribute('aria-label', 'Custom label');
  });
});
