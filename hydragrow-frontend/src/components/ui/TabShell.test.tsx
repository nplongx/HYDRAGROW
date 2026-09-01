import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TabShell } from './TabShell';

describe('TabShell', () => {
  const tabs = [
    { id: 'a', label: 'Điều khiển', content: <div>Nội dung A</div> },
    { id: 'b', label: 'Tự động hoá', content: <div>Nội dung B</div> },
  ];

  it('renders the active tab content and switches on click', () => {
    render(
      <TabShell
        title="Vận hành"
        subtitle="Bật/tắt thủ công hoặc chuyển sang tự động theo lịch."
        tabs={tabs}
        defaultTabId="a"
      />
    );

    expect(screen.getByText('Nội dung A')).toBeInTheDocument();
    expect(screen.queryByText('Nội dung B')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Tự động hoá' }));

    expect(screen.getByText('Nội dung B')).toBeInTheDocument();
    expect(screen.queryByText('Nội dung A')).not.toBeInTheDocument();
  });

  it('calls onTabChange when provided', () => {
    const onTabChange = vi.fn();
    render(<TabShell title="Vận hành" tabs={tabs} defaultTabId="a" onTabChange={onTabChange} />);
    fireEvent.click(screen.getByRole('tab', { name: 'Tự động hoá' }));
    expect(onTabChange).toHaveBeenCalledWith('b');
  });
});
