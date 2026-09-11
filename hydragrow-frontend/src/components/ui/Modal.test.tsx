import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Button } from './Button';
import { Modal } from './Modal';

describe('Modal', () => {
  it('không render gì khi open=false', () => {
    const { container } = render(<Modal open={false} onClose={vi.fn()} title="Cảnh báo" />);
    expect(container).toBeEmptyDOMElement();
  });

  it('hiển thị tiêu đề và nội dung khi open', () => {
    render(
      <Modal open onClose={vi.fn()} title="Cảnh báo">
        Nội dung chi tiết
      </Modal>,
    );
    expect(screen.getByRole('dialog', { name: 'Cảnh báo' })).toBeInTheDocument();
    expect(screen.getByText('Nội dung chi tiết')).toBeInTheDocument();
  });

  it('hiển thị footer action', () => {
    render(
      <Modal open onClose={vi.fn()} title="Xác nhận" footer={<Button>Đồng ý</Button>}>
        Xoá?
      </Modal>,
    );
    expect(screen.getByRole('button', { name: 'Đồng ý' })).toBeInTheDocument();
  });

  it('đóng khi bấm phím Escape', () => {
    const onClose = vi.fn();
    render(
      <Modal open onClose={onClose} title="Cảnh báo">
        Nội dung
      </Modal>,
    );
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('nút đóng gọi onClose', () => {
    const onClose = vi.fn();
    render(
      <Modal open onClose={onClose} title="Cảnh báo">
        Nội dung
      </Modal>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Đóng' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});