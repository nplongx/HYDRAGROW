import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import DesignLab from './DesignLab';

describe('DesignLab', () => {
  it('renders concept A from the default query state', () => {
    render(<MemoryRouter initialEntries={['/design-lab?concept=a']}><DesignLab /></MemoryRouter>);
    expect(screen.getByRole('heading', { name: 'Mọi hệ thống đang ổn định.' })).toBeInTheDocument();
    expect(screen.getByText('A/B/C là composition thật · cùng tokens · cùng information model')).toBeInTheDocument();
  });

  it('switches composition without leaving the design lab', () => {
    render(<MemoryRouter initialEntries={['/design-lab?concept=a']}><DesignLab /></MemoryRouter>);
    fireEvent.click(screen.getByRole('button', { name: 'C — Farm Overview' }));
    expect(screen.getByRole('heading', { name: 'Rau xà lách · Kệ A' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Phun sương' })).toBeInTheDocument();
  });
});
